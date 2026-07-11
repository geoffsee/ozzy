use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, Redirect};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use ozzy_core::{
    allows_api_key_permission, is_owner, parse_api_key, parse_bearer, validate_slug,
    ApiKeyPermission, MemberRole, Profile, Project, SecretMeta, SecretValue,
};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_sessions::Session;
use uuid::Uuid;
use worker::send;

use crate::auth::github::{finish_github_oauth, start_github_oauth};
use crate::auth::{AuthContext, AuthMethod, ProjectAccess};
use crate::crypto::{decrypt_secret, encrypt_secret, unwrap_dek};
use crate::db::api_keys::{create_api_key, list_api_keys, revoke_api_key};
use crate::db::profiles::get_profile_by_login;
use crate::db::projects::{
    add_member, create_project, get_member_role, get_project_for_profile_by_id, list_members,
    list_projects_for_profile, remove_member,
};
use crate::db::secrets::{delete_secret, get_secret_row, list_secrets, upsert_secret};
use crate::error::{bad_request, AppError, AppResult};
use crate::session_store::{SESSION_CSRF_TOKEN_KEY, SESSION_PROFILE_KEY};
use crate::state::AppState;

const CSRF_HEADER: &str = "x-csrf-token";

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/auth/github", get(auth_github))
        .route("/auth/github/callback", get(auth_github_callback))
        .route("/auth/logout", post(auth_logout))
        .route("/api/me", get(api_me))
        .route("/api/csrf", get(csrf_token_handler))
        .route("/api/projects", get(list_projects).post(create_project_handler))
        .route(
            "/api/projects/{slug}/members",
            get(list_members_handler)
                .post(add_member_handler)
                .delete(remove_member_handler),
        )
        .route("/api/keys", get(list_keys_handler).post(create_key_handler))
        .route("/api/keys/{id}", delete(revoke_key_handler))
        .route("/api/secrets/list", post(list_secrets_by_body_handler))
        .route("/api/secrets/read", post(get_secret_by_body_handler))
        .route("/api/secrets/write", put(put_secret_by_body_handler))
        .route("/api/secrets/delete", post(delete_secret_by_body_handler))
        .route("/v1/projects", get(list_projects))
        .route("/v1/projects/{slug}/secrets", get(list_secrets_handler))
        .route(
            "/v1/projects/{slug}/secrets/{key}",
            get(get_secret_handler)
                .put(put_secret_handler)
                .delete(delete_secret_handler),
        )
        .route("/v2/projects", get(list_projects))
        .route("/v2/secrets/list", post(list_secrets_by_body_handler))
        .route("/v2/secrets/read", post(get_secret_by_body_handler))
        .route("/v2/secrets/write", put(put_secret_by_body_handler))
        .route("/v2/secrets/delete", post(delete_secret_by_body_handler))
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
    Html(include_str!(env!("OZZY_UI_HTML_PATH")))
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
    session.cycle_id().await.map_err(|_| AppError::Internal)?;
    session
        .insert(SESSION_PROFILE_KEY, profile.id)
        .await
        .map_err(|_| AppError::Internal)?;
    session
        .insert(SESSION_CSRF_TOKEN_KEY, new_csrf_token())
        .await
        .map_err(|_| AppError::Internal)?;
    Ok(Redirect::to("/"))
}

#[send]
async fn auth_logout(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
) -> AppResult<StatusCode> {
    enforce_csrf(&auth, &session, &headers).await?;
    session.delete().await.map_err(|_| AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn api_me(auth: AuthContext) -> Json<Profile> {
    Json(auth.profile)
}

#[derive(Serialize)]
struct CsrfTokenResponse {
    token: String,
}

#[send]
async fn csrf_token_handler(
    auth: AuthContext,
    session: Session,
) -> AppResult<Json<CsrfTokenResponse>> {
    auth.require_session()?;
    let token = ensure_csrf_token(&session).await?;
    Ok(Json(CsrfTokenResponse { token }))
}

#[send]
async fn list_projects(
    auth: AuthContext,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<Project>>> {
    if let Some(ref key) = auth.api_key {
        let db = state.db()?;
        let mut projects = Vec::new();
        for scope in &key.scopes {
            if get_project_for_profile_by_id(&db, &auth.profile.id, &scope.project_id)
                .await?
                .is_some()
            {
                if let Some(p) =
                    crate::db::projects::get_project_by_id(&db, &scope.project_id).await?
                {
                    projects.push(p);
                }
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
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> AppResult<(StatusCode, Json<Project>)> {
    enforce_csrf(&auth, &session, &headers).await?;
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
async fn get_project_exists(state: &AppState, profile_id: &str, slug: &str) -> AppResult<bool> {
    Ok(
        crate::db::projects::get_project_for_profile(&state.db()?, profile_id, slug)
            .await?
            .is_some(),
    )
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
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Json(body): Json<MemberBody>,
) -> AppResult<StatusCode> {
    enforce_csrf(&auth, &session, &headers).await?;
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
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Query(q): Query<RemoveMemberQuery>,
) -> AppResult<StatusCode> {
    enforce_csrf(&auth, &session, &headers).await?;
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
) -> AppResult<Json<Vec<ozzy_core::ApiKeyInfo>>> {
    auth.require_session()?;
    Ok(Json(list_api_keys(&state.db()?, &auth.profile.id).await?))
}

#[send]
async fn create_key_handler(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<CreateKeyBody>,
) -> AppResult<(StatusCode, Json<ozzy_core::CreateApiKeyResponse>)> {
    enforce_csrf(&auth, &session, &headers).await?;
    auth.require_session()?;
    if body.name.trim().is_empty() {
        return Err(bad_request("name required"));
    }
    let db = state.db()?;
    let mut scopes = Vec::new();
    for s in body.scopes {
        let perm = ApiKeyPermission::parse(&s.permission)
            .ok_or_else(|| bad_request("invalid permission"))?;
        let project = get_project_for_profile_by_id(&db, &auth.profile.id, &s.project_id)
            .await?
            .ok_or(AppError::NotFound)?;
        let owner = is_owner(&auth.profile.id, &project.owner_profile_id);
        let role = get_member_role(&db, &project.id, &auth.profile.id).await?;
        if !allows_api_key_permission(owner, role, perm) {
            return Err(bad_request("insufficient permission for project scope"));
        }
        scopes.push((s.project_id, perm));
    }
    let created = create_api_key(
        &db,
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
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> AppResult<StatusCode> {
    enforce_csrf(&auth, &session, &headers).await?;
    auth.require_session()?;
    if !revoke_api_key(&state.db()?, &auth.profile.id, &id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct SecretProjectBody {
    project: String,
}

#[send]
async fn list_secrets_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> AppResult<Json<Vec<SecretMeta>>> {
    list_secrets_for_project(&auth, &state, &slug).await
}

#[send]
async fn list_secrets_by_body_handler(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<SecretProjectBody>,
) -> AppResult<Json<Vec<SecretMeta>>> {
    enforce_csrf(&auth, &session, &headers).await?;
    list_secrets_for_project(&auth, &state, &body.project).await
}

#[send]
async fn list_secrets_for_project(
    auth: &AuthContext,
    state: &AppState,
    project: &str,
) -> AppResult<Json<Vec<SecretMeta>>> {
    let access = auth.project_access(state, project).await?;
    if !access.can_read {
        return Err(AppError::NotFound);
    }
    Ok(Json(list_secrets(&state.db()?, &access.project.id).await?))
}

#[derive(Deserialize)]
struct SecretSelectorBody {
    project: String,
    key: String,
}

#[send]
async fn get_secret_handler(
    auth: AuthContext,
    State(state): State<AppState>,
    axum::extract::Path((slug, key)): axum::extract::Path<(String, String)>,
) -> AppResult<Json<SecretValue>> {
    get_secret_for_project_key(&auth, &state, &slug, &key).await
}

#[send]
async fn get_secret_by_body_handler(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<SecretSelectorBody>,
) -> AppResult<Json<SecretValue>> {
    enforce_csrf(&auth, &session, &headers).await?;
    get_secret_for_project_key(&auth, &state, &body.project, &body.key).await
}

#[send]
async fn get_secret_for_project_key(
    auth: &AuthContext,
    state: &AppState,
    project: &str,
    key: &str,
) -> AppResult<Json<SecretValue>> {
    let access = auth.project_access(state, project).await?;
    if !access.can_read {
        return Err(AppError::NotFound);
    }
    let value = read_secret(state, &access, key).await?;
    Ok(Json(value))
}

#[derive(Deserialize)]
struct PutSecretByBody {
    project: String,
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct PutSecretBody {
    value: String,
}

#[send]
async fn put_secret_handler(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    axum::extract::Path((slug, key)): axum::extract::Path<(String, String)>,
    Json(body): Json<PutSecretBody>,
) -> AppResult<Json<SecretValue>> {
    enforce_csrf(&auth, &session, &headers).await?;
    put_secret_for_project_key(&auth, &state, &slug, &key, &body.value).await
}

#[send]
async fn put_secret_by_body_handler(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<PutSecretByBody>,
) -> AppResult<Json<SecretValue>> {
    enforce_csrf(&auth, &session, &headers).await?;
    put_secret_for_project_key(&auth, &state, &body.project, &body.key, &body.value).await
}

#[send]
async fn put_secret_for_project_key(
    auth: &AuthContext,
    state: &AppState,
    project: &str,
    key: &str,
    value: &str,
) -> AppResult<Json<SecretValue>> {
    let access = auth.project_access(state, project).await?;
    if !access.can_write {
        return Err(AppError::NotFound);
    }
    let version = write_secret(state, &access, key, value, &auth.profile.id).await?;
    Ok(Json(SecretValue {
        key_name: key.to_string(),
        value: value.to_string(),
        version,
    }))
}

#[send]
async fn delete_secret_handler(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    axum::extract::Path((slug, key)): axum::extract::Path<(String, String)>,
) -> AppResult<StatusCode> {
    enforce_csrf(&auth, &session, &headers).await?;
    delete_secret_for_project_key(&auth, &state, &slug, &key).await
}

#[send]
async fn delete_secret_by_body_handler(
    auth: AuthContext,
    headers: HeaderMap,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<SecretSelectorBody>,
) -> AppResult<StatusCode> {
    enforce_csrf(&auth, &session, &headers).await?;
    delete_secret_for_project_key(&auth, &state, &body.project, &body.key).await
}

#[send]
async fn delete_secret_for_project_key(
    auth: &AuthContext,
    state: &AppState,
    project: &str,
    key: &str,
) -> AppResult<StatusCode> {
    let access = auth.project_access(state, project).await?;
    if !access.can_write {
        return Err(AppError::NotFound);
    }
    if !delete_secret(&state.db()?, &access.project.id, key).await? {
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

fn new_csrf_token() -> String {
    Uuid::new_v4().to_string()
}

fn auth_uses_api_key(auth: &AuthContext, auth_header: Option<&str>) -> bool {
    if matches!(auth.method, AuthMethod::ApiKey) {
        return true;
    }
    parse_bearer(auth_header).and_then(parse_api_key).is_some()
}

fn validate_csrf_values(provided: Option<&str>, expected: Option<&str>) -> AppResult<()> {
    let Some(expected) = expected else {
        return Err(bad_request("csrf token missing"));
    };
    let Some(provided) = provided else {
        return Err(bad_request("csrf token missing"));
    };
    if bool::from(provided.as_bytes().ct_eq(expected.as_bytes())) {
        Ok(())
    } else {
        Err(bad_request("invalid csrf token"))
    }
}

#[send]
async fn ensure_csrf_token(session: &Session) -> AppResult<String> {
    if let Some(existing) = session
        .get::<String>(SESSION_CSRF_TOKEN_KEY)
        .await
        .map_err(|_| AppError::Internal)?
    {
        return Ok(existing);
    }

    let token = new_csrf_token();
    session
        .insert(SESSION_CSRF_TOKEN_KEY, token.clone())
        .await
        .map_err(|_| AppError::Internal)?;
    Ok(token)
}

#[send]
async fn enforce_csrf(auth: &AuthContext, session: &Session, headers: &HeaderMap) -> AppResult<()> {
    if auth_uses_api_key(
        auth,
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
    ) {
        return Ok(());
    }

    auth.require_session()?;

    let expected = session
        .get::<String>(SESSION_CSRF_TOKEN_KEY)
        .await
        .map_err(|_| AppError::Internal)?;
    let provided = headers
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok());
    validate_csrf_values(provided, expected.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_auth(method: AuthMethod) -> AuthContext {
        AuthContext {
            profile: Profile {
                id: "profile-1".into(),
                github_id: 1,
                login: "user".into(),
                name: Some("User".into()),
                avatar_url: Some("https://example.com/a.png".into()),
            },
            method,
            api_key: None,
        }
    }

    #[test]
    fn csrf_allows_api_key_requests_without_token() {
        let auth = fake_auth(AuthMethod::ApiKey);
        assert!(auth_uses_api_key(&auth, None));
    }

    #[test]
    fn csrf_rejects_missing_token() {
        let err =
            validate_csrf_values(None, Some("expected")).expect_err("should reject missing token");
        assert!(matches!(err, AppError::BadRequest(message) if message == "csrf token missing"));
    }

    #[test]
    fn csrf_rejects_invalid_token() {
        let err = validate_csrf_values(Some("wrong"), Some("expected"))
            .expect_err("should reject invalid token");
        assert!(matches!(err, AppError::BadRequest(message) if message == "invalid csrf token"));
    }

    #[test]
    fn csrf_accepts_matching_token() {
        validate_csrf_values(Some("expected"), Some("expected"))
            .expect("should accept matching token");
    }
}

#[derive(Serialize)]
struct GitHubTestUser {
    id: i64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}
