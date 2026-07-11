use base64::Engine;
use ozzy_core::Profile;
use serde::Deserialize;
use worker::send;

use crate::crypto::{random_bytes, sha256_raw};
use crate::db::oauth::{cleanup_expired_oauth_states, consume_oauth_state, create_oauth_state};
use crate::db::profiles::upsert_profile;
use crate::error::{bad_request, internal, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GitHubUser {
    id: i64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[send]
pub async fn start_github_oauth(state: &AppState) -> AppResult<(String, String)> {
    let _ = cleanup_expired_oauth_states(&state.db()?).await;
    let verifier_bytes = random_bytes(32)?;
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);
    let challenge_bytes = sha256_raw(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&challenge_bytes);
    let oauth_state = uuid::Uuid::new_v4().to_string();
    let expires_at = (time::OffsetDateTime::now_utc() + time::Duration::minutes(10))
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| internal("time"))?;

    create_oauth_state(&state.db()?, &oauth_state, &verifier, &expires_at).await?;

    let client_id = state.github_client_id()?;
    let base = state.base_url()?;
    let redirect_uri = format!("{base}/auth/github/callback");
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&oauth_state),
        urlencoding::encode(&challenge),
    );

    Ok((url, oauth_state))
}

#[send]
pub async fn finish_github_oauth(
    state: &AppState,
    code: &str,
    oauth_state: &str,
) -> AppResult<Profile> {
    let verifier = consume_oauth_state(&state.db()?, oauth_state)
        .await?
        .ok_or_else(|| bad_request("invalid oauth state"))?;

    let token = exchange_code(state, code, &verifier).await?;
    let user = fetch_github_user(state, &token).await?;
    let profile = upsert_profile(
        &state.db()?,
        user.id,
        &user.login,
        user.name.as_deref(),
        user.avatar_url.as_deref(),
    )
    .await?;

    Ok(profile)
}

#[send]
async fn exchange_code(state: &AppState, code: &str, verifier: &str) -> AppResult<String> {
    let base = state.github_api_base()?;
    let url = if state.test_mode() {
        format!("{base}/login/oauth/access_token")
    } else {
        "https://github.com/login/oauth/access_token".to_string()
    };

    let body = format!(
        "client_id={}&client_secret={}&code={}&redirect_uri={}&code_verifier={}",
        urlencoding::encode(&state.github_client_id()?),
        urlencoding::encode(&state.github_client_secret()?),
        urlencoding::encode(code),
        urlencoding::encode(&format!("{}/auth/github/callback", state.base_url()?)),
        urlencoding::encode(verifier),
    );

    let mut init = worker::RequestInit::new();
    init.with_method(worker::Method::Post);
    init.with_body(Some(body.into()));
    let headers = worker::Headers::new();
    headers
        .set("Accept", "application/json")
        .map_err(|e| internal(e))?;
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|e| internal(e))?;
    init.with_headers(headers);

    let request = worker::Request::new_with_init(&url, &init).map_err(|e| internal(e))?;
    let mut resp = worker::Fetch::Request(request)
        .send()
        .await
        .map_err(|e| internal(e))?;

    if resp.status_code() >= 400 {
        return Err(bad_request("token exchange failed"));
    }

    let parsed: GitHubTokenResponse = resp.json().await.map_err(|e| internal(e))?;
    Ok(parsed.access_token)
}

#[send]
async fn fetch_github_user(state: &AppState, token: &str) -> AppResult<GitHubUser> {
    let base = state.github_api_base()?;
    let url = if state.test_mode() {
        format!("{base}/user")
    } else {
        "https://api.github.com/user".to_string()
    };

    let mut init = worker::RequestInit::new();
    init.with_method(worker::Method::Get);
    let headers = worker::Headers::new();
    headers
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| internal(e))?;
    headers
        .set("Accept", "application/json")
        .map_err(|e| internal(e))?;
    headers
        .set("User-Agent", "ozzy-api")
        .map_err(|e| internal(e))?;
    init.with_headers(headers);

    let request = worker::Request::new_with_init(&url, &init).map_err(|e| internal(e))?;
    let mut resp = worker::Fetch::Request(request)
        .send()
        .await
        .map_err(|e| internal(e))?;

    if resp.status_code() >= 400 {
        return Err(bad_request("user fetch failed"));
    }

    resp.json().await.map_err(|e| internal(e))
}
