use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use oz_core::{
    can_read_api_key, can_write_api_key, effective_admin, effective_read, effective_write,
    is_owner, parse_api_key, parse_bearer, Profile,
};
use tower_sessions::Session;
use worker::send;

use crate::db::api_keys::{resolve_api_key, ResolvedApiKey};
use crate::db::projects::{
    get_member_role, get_project_for_profile, get_project_for_profile_by_id,
};
use crate::db::ProjectCryptoRow;
use crate::error::{AppError, AppResult};
use crate::session_store::SESSION_PROFILE_KEY;
use crate::state::AppState;

pub mod github;

#[derive(Clone, Debug)]
pub enum AuthMethod {
    Session,
    ApiKey,
}

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub profile: Profile,
    pub method: AuthMethod,
    pub api_key: Option<ResolvedApiKey>,
}

#[derive(Clone, Debug)]
pub struct ProjectAccess {
    pub project: ProjectCryptoRow,
    pub can_read: bool,
    pub can_write: bool,
    pub can_admin: bool,
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let app = AppState::from_ref(state);
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(str::to_string);
        let session = parts.extensions.get::<Session>().cloned();
        authenticate(app, auth_header, session)
    }
}

#[send]
async fn authenticate(
    app: AppState,
    auth_header: Option<String>,
    session: Option<Session>,
) -> Result<AuthContext, AppError> {
    let db = app.db().map_err(|_| AppError::Internal)?;

    if let Some(auth) = auth_header.as_deref() {
        if let Some(raw) = parse_bearer(Some(auth)) {
            if parse_api_key(raw).is_some() {
                let pepper = app.api_key_pepper().map_err(|_| AppError::Internal)?;
                let resolved = resolve_api_key(&db, raw, &pepper)
                    .await
                    .map_err(|_| AppError::Internal)?
                    .ok_or(AppError::Unauthorized)?;
                let profile = crate::db::profiles::get_profile_by_id(&db, &resolved.profile_id)
                    .await
                    .map_err(|_| AppError::Internal)?
                    .ok_or(AppError::Unauthorized)?;
                return Ok(AuthContext {
                    profile,
                    method: AuthMethod::ApiKey,
                    api_key: Some(resolved),
                });
            }
        }
    }

    let session = session.ok_or(AppError::Internal)?;
    let profile_id: String = session
        .get(SESSION_PROFILE_KEY)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;
    let profile = crate::db::profiles::get_profile_by_id(&db, &profile_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    Ok(AuthContext {
        profile,
        method: AuthMethod::Session,
        api_key: None,
    })
}

impl AuthContext {
    pub fn require_session(&self) -> AppResult<()> {
        if matches!(self.method, AuthMethod::Session) {
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }

    pub async fn project_access(
        &self,
        state: &AppState,
        project_ref: &str,
    ) -> AppResult<ProjectAccess> {
        project_access_for(self, state, project_ref).await
    }
}

fn is_project_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
}

#[send]
async fn resolve_project_for_profile(
    db: &worker::D1Database,
    profile_id: &str,
    project_ref: &str,
) -> AppResult<Option<ProjectCryptoRow>> {
    if is_project_id(project_ref) {
        get_project_for_profile_by_id(db, profile_id, project_ref).await
    } else {
        get_project_for_profile(db, profile_id, project_ref).await
    }
}

#[send]
async fn project_access_for(
    auth: &AuthContext,
    state: &AppState,
    project_ref: &str,
) -> AppResult<ProjectAccess> {
    let db = state.db()?;
    let project = resolve_project_for_profile(&db, &auth.profile.id, project_ref)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(ref key) = auth.api_key {
        let scope = key.scopes.iter().find(|s| s.project_id == project.id);
        let Some(scope) = scope else {
            return Err(AppError::NotFound);
        };
        return Ok(ProjectAccess {
            can_read: can_read_api_key(scope.permission),
            can_write: can_write_api_key(scope.permission),
            can_admin: false,
            project,
        });
    }

    let owner = is_owner(&auth.profile.id, &project.owner_profile_id);
    let role = get_member_role(&db, &project.id, &auth.profile.id).await?;
    Ok(ProjectAccess {
        can_read: effective_read(owner, role),
        can_write: effective_write(owner, role),
        can_admin: effective_admin(owner, role),
        project,
    })
}
