#![allow(dead_code)]

use ozzy_core::{ApiKeyInfo, ApiKeyPermission, ApiKeyScope, CreateApiKeyResponse, MemberRole, Profile, Project, SecretMeta, SecretValue};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    paths(
        github_oauth,
        github_oauth_callback,
        auth_logout,
        api_me,
        api_csrf,
        api_list_projects,
        api_create_project,
        api_list_members,
        api_add_member,
        api_remove_member,
        api_list_keys,
        api_create_key,
        api_revoke_key,
        api_list_secrets,
        api_read_secret,
        api_write_secret,
        api_delete_secret,
        v1_list_projects,
        v1_list_project_secrets,
        v1_get_secret,
        v1_write_secret,
        v1_delete_secret,
        v2_list_projects,
        v2_list_secrets,
        v2_read_secret,
        v2_write_secret,
        v2_delete_secret
    ),
    components(
        schemas(
            ApiKeyInfo,
            ApiKeyScope,
            ApiKeyPermission,
            CreateApiKeyRequest,
            CreateApiKeyResponse,
            CreateProjectBody,
            CsrfTokenResponse,
            MemberRole,
            PathSecretWriteBody,
            Project,
            ProjectMember,
            ProjectMemberBody,
            ProjectSelectorBody,
            Profile,
            SecretMeta,
            SecretSelectorBody,
            SecretValue,
            SecretWriteBody
        )
    )
)]
pub struct ApiDoc;

pub fn build_openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[utoipa::path(
    get,
    path = "/auth/github",
    tag = "auth",
    responses((status = 302, description = "Redirects to GitHub OAuth authorization"))
)]
pub fn github_oauth() {}

#[utoipa::path(
    get,
    path = "/auth/github/callback",
    tag = "auth",
    params(("code" = String, Query, description = "OAuth authorization code"), ("state" = String, Query, description = "OAuth state")),
    responses((status = 302, description = "Completes sign-in flow"))
)]
pub fn github_oauth_callback() {}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Signed out"),
        (status = 403, description = "Invalid CSRF token")
    )
)]
pub fn auth_logout() {}

#[utoipa::path(
    get,
    path = "/api/me",
    tag = "api",
    responses((status = 200, description = "Current profile", body = Profile))
)]
pub fn api_me() {}

#[utoipa::path(
    get,
    path = "/api/csrf",
    tag = "api",
    responses((status = 200, description = "CSRF token", body = CsrfTokenResponse))
)]
pub fn api_csrf() {}

#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "api",
    responses((status = 200, description = "Visible projects", body = Vec<Project>))
)]
pub fn api_list_projects() {}

#[utoipa::path(
    post,
    path = "/api/projects",
    tag = "api",
    request_body = CreateProjectBody,
    responses(
        (status = 201, description = "Created project", body = Project),
        (status = 400, description = "Invalid request")
    )
)]
pub fn api_create_project() {}

#[utoipa::path(
    get,
    path = "/api/projects/{slug}/members",
    tag = "api",
    params(("slug" = String, Path, description = "Project slug")),
    responses((status = 200, description = "Project members", body = Vec<ProjectMember>))
)]
pub fn api_list_members() {}

#[utoipa::path(
    post,
    path = "/api/projects/{slug}/members",
    tag = "api",
    params(("slug" = String, Path, description = "Project slug")),
    request_body = ProjectMemberBody,
    responses((status = 201, description = "Member added"))
)]
pub fn api_add_member() {}

#[utoipa::path(
    delete,
    path = "/api/projects/{slug}/members",
    tag = "api",
    params(
        ("slug" = String, Path, description = "Project slug"),
        ("login" = String, Query, description = "Member login")
    ),
    responses((status = 204, description = "Member removed"))
)]
pub fn api_remove_member() {}

#[utoipa::path(
    get,
    path = "/api/keys",
    tag = "api",
    responses((status = 200, description = "API keys", body = Vec<ApiKeyInfo>))
)]
pub fn api_list_keys() {}

#[utoipa::path(
    post,
    path = "/api/keys",
    tag = "api",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "Created key", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub fn api_create_key() {}

#[utoipa::path(
    delete,
    path = "/api/keys/{id}",
    tag = "api",
    params(("id" = String, Path, description = "API key id")),
    responses((status = 204, description = "Key revoked"))
)]
pub fn api_revoke_key() {}

#[utoipa::path(
    post,
    path = "/api/secrets/list",
    tag = "api",
    request_body = ProjectSelectorBody,
    responses((status = 200, description = "Secret metadata", body = Vec<SecretMeta>))
)]
pub fn api_list_secrets() {}

#[utoipa::path(
    post,
    path = "/api/secrets/read",
    tag = "api",
    request_body = SecretSelectorBody,
    responses((status = 200, description = "Secret value", body = SecretValue))
)]
pub fn api_read_secret() {}

#[utoipa::path(
    put,
    path = "/api/secrets/write",
    tag = "api",
    request_body = SecretWriteBody,
    responses((status = 200, description = "Stored secret", body = SecretValue))
)]
pub fn api_write_secret() {}

#[utoipa::path(
    post,
    path = "/api/secrets/delete",
    tag = "api",
    request_body = SecretSelectorBody,
    responses((status = 204, description = "Secret deleted"))
)]
pub fn api_delete_secret() {}

#[utoipa::path(
    get,
    path = "/v1/projects",
    tag = "api",
    responses((status = 200, description = "Projects", body = Vec<Project>))
)]
pub fn v1_list_projects() {}

#[utoipa::path(
    get,
    path = "/v1/projects/{slug}/secrets",
    tag = "api",
    params(("slug" = String, Path, description = "Project slug")),
    responses((status = 200, description = "Secret metadata", body = Vec<SecretMeta>))
)]
pub fn v1_list_project_secrets() {}

#[utoipa::path(
    get,
    path = "/v1/projects/{slug}/secrets/{key}",
    tag = "api",
    params(
        ("slug" = String, Path, description = "Project slug"),
        ("key" = String, Path, description = "Secret key")
    ),
    responses((status = 200, description = "Secret value", body = SecretValue))
)]
pub fn v1_get_secret() {}

#[utoipa::path(
    put,
    path = "/v1/projects/{slug}/secrets/{key}",
    tag = "api",
    params(
        ("slug" = String, Path, description = "Project slug"),
        ("key" = String, Path, description = "Secret key")
    ),
    request_body = PathSecretWriteBody,
    responses((status = 200, description = "Stored secret", body = SecretValue))
)]
pub fn v1_write_secret() {}

#[utoipa::path(
    delete,
    path = "/v1/projects/{slug}/secrets/{key}",
    tag = "api",
    params(
        ("slug" = String, Path, description = "Project slug"),
        ("key" = String, Path, description = "Secret key")
    ),
    responses((status = 204, description = "Secret deleted"))
)]
pub fn v1_delete_secret() {}

#[utoipa::path(
    get,
    path = "/v2/projects",
    tag = "api",
    responses((status = 200, description = "Projects", body = Vec<Project>))
)]
pub fn v2_list_projects() {}

#[utoipa::path(
    post,
    path = "/v2/secrets/list",
    tag = "api",
    request_body = ProjectSelectorBody,
    responses((status = 200, description = "Secret metadata", body = Vec<SecretMeta>))
)]
pub fn v2_list_secrets() {}

#[utoipa::path(
    post,
    path = "/v2/secrets/read",
    tag = "api",
    request_body = SecretSelectorBody,
    responses((status = 200, description = "Secret value", body = SecretValue))
)]
pub fn v2_read_secret() {}

#[utoipa::path(
    put,
    path = "/v2/secrets/write",
    tag = "api",
    request_body = SecretWriteBody,
    responses((status = 200, description = "Stored secret", body = SecretValue))
)]
pub fn v2_write_secret() {}

#[utoipa::path(
    post,
    path = "/v2/secrets/delete",
    tag = "api",
    request_body = SecretSelectorBody,
    responses((status = 204, description = "Secret deleted"))
)]
pub fn v2_delete_secret() {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateProjectBody {
    pub slug: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectMember {
    pub profile_id: String,
    pub login: String,
    pub role: MemberRole,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectMemberBody {
    pub login: String,
    pub role: MemberRole,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Vec<KeyScopeBody>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct KeyScopeBody {
    pub project_id: String,
    pub permission: ApiKeyPermission,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectSelectorBody {
    pub project: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SecretSelectorBody {
    pub project: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SecretWriteBody {
    pub project: String,
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PathSecretWriteBody {
    pub value: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CsrfTokenResponse {
    pub token: String,
}
