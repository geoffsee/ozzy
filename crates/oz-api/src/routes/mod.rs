use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, Redirect};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use oz_core::{
    validate_slug, ApiKeyPermission, MemberRole, Profile, Project, SecretMeta, SecretValue,
};
use serde::{Deserialize, Serialize};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_sessions::Session;
use worker::send;

use crate::auth::github::{finish_github_oauth, start_github_oauth};
use crate::auth::{AuthContext, ProjectAccess};
use crate::session_store::SESSION_PROFILE_KEY;
use crate::crypto::{decrypt_secret, encrypt_secret, unwrap_dek};
use crate::db::api_keys::{create_api_key, list_api_keys, revoke_api_key};
use crate::db::profiles::get_profile_by_login;
use crate::db::projects::{
    add_member, create_project, list_members, list_projects_for_profile, remove_member,
};
use crate::db::secrets::{delete_secret, get_secret_row, list_secrets, upsert_secret};
use crate::error::{bad_request, AppError, AppResult};
use crate::state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/auth/github", get(auth_github))
        .route("/auth/github/callback", get(auth_github_callback))
        .route("/auth/logout", post(auth_logout))
        .route("/api/me", get(api_me))
        .route("/api/projects", get(list_projects).post(create_project_handler))
        .route(
            "/api/projects/{slug}/members",
            get(list_members_handler)
                .post(add_member_handler)
                .delete(remove_member_handler),
        )
        .route("/api/keys", get(list_keys_handler).post(create_key_handler))
        .route("/api/keys/{id}", delete(revoke_key_handler))
        .route("/api/projects/{slug}/secrets", get(list_secrets_handler))
        .route(
            "/api/projects/{slug}/secrets/{key}",
            get(get_secret_handler)
                .put(put_secret_handler)
                .delete(delete_secret_handler),
        )
        .route("/v1/projects", get(list_projects))
        .route("/v1/projects/{slug}/secrets", get(list_secrets_handler))
        .route(
            "/v1/projects/{slug}/secrets/{key}",
            get(get_secret_handler)
                .put(put_secret_handler)
                .delete(delete_secret_handler),
        )
        .route("/test/github/login/oauth/access_token", post(test_github_token))
        .route("/test/github/user", get(test_github_user))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'",
            ),
        ))
}

async fn index() -> Html<&'static str> {
    Html(include_str!(env!("OZ_UI_HTML_PATH")))
}

#[send]
async fn auth_github(State(state): State<AppState>) -> AppResult<Redirect> {
    let (url, _) = start_github_oauth(&state).await?;
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

#[send]
async fn auth_github_callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
    session: Session,
) -> AppResult<Redirect> {
    let profile = finish_github_oauth(&state, &q.code, &q.state).await?;
    session
        .cycle_id()
        .await
        .map_err(|_| AppError::Internal)?;
    session
        .insert(SESSION_PROFILE_KEY, profile.id)
        .await
        .map_err(|_| AppError::Internal)?;
    Ok(Redirect::to("/"))
}

#[send]
async fn auth_logout(session: Session) -> AppResult<StatusCode> {
    session.delete().await.map_err(|_| AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn api_me(auth: AuthContext) -> Json<Profile> {
    Json(auth.profile)
}

#[send]
async fn list_projects(auth: AuthContext, State(state): State<AppState>) -> AppResult<Json<Vec<Project>>> {
    if let Some(ref key) = auth.api_key {
        let mut projects = Vec::new();
        for scope in &key.scopes {
            if let Some(p) = crate::db::projects::get_project_by_id(&state.db()?, &scope.project_id).await? {
                projects.push(p);
            }
        }
        return Ok(Json(projects));
    }
    let projects = list_projects_for_profile(&state.db()?, &auth.profile.id).await?;
    Ok(Json(projects))
}

#[derive(Deserialize)]
struct CreateProjectBody {
    slug: String,
    name: String,
}

#[send]
async fn create_project_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> AppResult<(StatusCode, Json<Project>)> {
    auth.require_session()?;
    validate_slug(&body.slug).map_err(|e| bad_request(e))?;
    if body.name.trim().is_empty() {
        return Err(bad_request("name required"));
    }
    let master_key = state.master_key()?;
    match create_project(
        &state.db()?,
        &auth.profile.id,
        &body.slug,
        body.name.trim(),
        &master_key,
    )
    .await
    {
        Ok(p) => Ok((StatusCode::CREATED, Json(p))),
        Err(AppError::Internal) => {
            if get_project_exists(&state, &auth.profile.id, &body.slug).await? {
                Err(AppError::Conflict("project slug already exists".into()))
            } else {
                Err(AppError::Internal)
            }
        }
        Err(e) => Err(e),
    }
}

#[send]
async fn get_project_exists(
    state: &AppState,
    profile_id: &str,
    slug: &str,
) -> AppResult<bool> {
    Ok(crate::db::projects::get_project_for_profile(&state.db()?, profile_id, slug)
        .await?
        .is_some())
}

#[derive(Deserialize)]
struct MemberBody {
    login: String,
    role: String,
}

#[send]
async fn list_members_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    auth.require_session()?;
    let access = auth.project_access(&state, &slug).await?;
    if !access.can_admin {
        return Err(AppError::NotFound);
    }
    let members = list_members(&state.db()?, &access.project.id).await?;
    Ok(Json(members))
}

#[send]
async fn add_member_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Json(body): Json<MemberBody>,
) -> AppResult<StatusCode> {
    auth.require_session()?;
    let access = auth.project_access(&state, &slug).await?;
    if !access.can_admin {
        return Err(AppError::NotFound);
    }
    let role = MemberRole::parse(&body.role).ok_or_else(|| bad_request("invalid role"))?;
    let member = get_profile_by_login(&state.db()?, &body.login)
        .await?
        .ok_or(AppError::NotFound)?;
    add_member(&state.db()?, &access.project.id, &member.id, role).await?;
    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
struct RemoveMemberQuery {
    login: String,
}

#[send]
async fn remove_member_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Query(q): Query<RemoveMemberQuery>,
) -> AppResult<StatusCode> {
    auth.require_session()?;
    let access = auth.project_access(&state, &slug).await?;
    if !access.can_admin {
        return Err(AppError::NotFound);
    }
    let member = get_profile_by_login(&state.db()?, &q.login)
        .await?
        .ok_or(AppError::NotFound)?;
    remove_member(&state.db()?, &access.project.id, &member.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct CreateKeyBody {
    name: String,
    scopes: Vec<KeyScopeBody>,
}

#[derive(Deserialize)]
struct KeyScopeBody {
    project_id: String,
    permission: String,
}

#[send]
async fn list_keys_handler(
    auth: AuthContext,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<oz_core::ApiKeyInfo>>> {
    auth.require_session()?;
    Ok(Json(list_api_keys(&state.db()?, &auth.profile.id).await?))
}

#[send]
async fn create_key_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(body): Json<CreateKeyBody>,
) -> AppResult<(StatusCode, Json<oz_core::CreateApiKeyResponse>)> {
    auth.require_session()?;
    if body.name.trim().is_empty() {
        return Err(bad_request("name required"));
    }
    let mut scopes = Vec::new();
    for s in body.scopes {
        let perm = ApiKeyPermission::parse(&s.permission)
            .ok_or_else(|| bad_request("invalid permission"))?;
        scopes.push((s.project_id, perm));
    }
    let created = create_api_key(
        &state.db()?,
        &auth.profile.id,
        body.name.trim(),
        &scopes,
        &state.api_key_pepper()?,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[send]
async fn revoke_key_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> AppResult<StatusCode> {
    auth.require_session()?;
    if !revoke_api_key(&state.db()?, &auth.profile.id, &id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[send]
async fn list_secrets_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> AppResult<Json<Vec<SecretMeta>>> {
    let access = auth.project_access(&state, &slug).await?;
    if !access.can_read {
        return Err(AppError::NotFound);
    }
    Ok(Json(list_secrets(&state.db()?, &access.project.id).await?))
}

#[send]
async fn get_secret_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path((slug, key)): axum::extract::Path<(String, String)>,
) -> AppResult<Json<SecretValue>> {
    let access = auth.project_access(&state, &slug).await?;
    if !access.can_read {
        return Err(AppError::NotFound);
    }
    let value = read_secret(&state, &access, &key).await?;
    Ok(Json(value))
}

#[derive(Deserialize)]
struct PutSecretBody {
    value: String,
}

#[send]
async fn put_secret_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path((slug, key)): axum::extract::Path<(String, String)>,
    Json(body): Json<PutSecretBody>,
) -> AppResult<Json<SecretValue>> {
    let access = auth.project_access(&state, &slug).await?;
    if !access.can_write {
        return Err(AppError::NotFound);
    }
    let version = write_secret(&state, &access, &key, &body.value, &auth.profile.id).await?;
    Ok(Json(SecretValue {
        key_name: key,
        value: body.value,
        version,
    }))
}

#[send]
async fn delete_secret_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path((slug, key)): axum::extract::Path<(String, String)>,
) -> AppResult<StatusCode> {
    let access = auth.project_access(&state, &slug).await?;
    if !access.can_write {
        return Err(AppError::NotFound);
    }
    if !delete_secret(&state.db()?, &access.project.id, &key).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[send]
async fn read_secret(
    state: &AppState,
    access: &ProjectAccess,
    key: &str,
) -> AppResult<SecretValue> {
    let row = get_secret_row(&state.db()?, &access.project.id, key)
        .await?
        .ok_or(AppError::NotFound)?;
    let dek = unwrap_dek(
        &state.master_key()?,
        &access.project.wrapped_dek,
        &access.project.dek_wrap_nonce,
    )?;
    let value = decrypt_secret(&dek, &row.0, &row.1).map_err(|_| AppError::Internal)?;
    Ok(SecretValue {
        key_name: key.to_string(),
        value,
        version: row.2,
    })
}

#[send]
async fn write_secret(
    state: &AppState,
    access: &ProjectAccess,
    key: &str,
    value: &str,
    profile_id: &str,
) -> AppResult<i64> {
    let dek = unwrap_dek(
        &state.master_key()?,
        &access.project.wrapped_dek,
        &access.project.dek_wrap_nonce,
    )?;
    let (ciphertext, nonce) = encrypt_secret(&dek, value).map_err(|_| AppError::Internal)?;
    upsert_secret(
        &state.db()?,
        &access.project.id,
        key,
        &ciphertext,
        &nonce,
        profile_id,
    )
    .await
}

#[derive(Serialize)]
struct TestTokenResponse {
    access_token: String,
    token_type: String,
}

#[send]
async fn test_github_token(
    State(state): State<AppState>,
    body: String,
) -> AppResult<Json<TestTokenResponse>> {
    if !state.test_mode() {
        return Err(AppError::NotFound);
    }
    let _ = body;
    Ok(Json(TestTokenResponse {
        access_token: "test-access-token".into(),
        token_type: "bearer".into(),
    }))
}

#[send]
async fn test_github_user(State(state): State<AppState>) -> AppResult<Json<GitHubTestUser>> {
    if !state.test_mode() {
        return Err(AppError::NotFound);
    }
    Ok(Json(GitHubTestUser {
        id: 42,
        login: "test-user".into(),
        name: Some("Test User".into()),
        avatar_url: Some("https://example.com/avatar.png".into()),
    }))
}

#[derive(Serialize)]
struct GitHubTestUser {
    id: i64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}
