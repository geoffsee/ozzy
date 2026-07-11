mod auth;
mod crypto;
mod db;
mod error;
mod openapi;
mod routes;
mod session_store;
mod state;

use axum::Router;
use time::Duration;
use tower_service::Service;
use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};
use worker::*;

use session_store::D1SessionStore;
use state::AppState;

fn router(env: Env) -> Router {
    let session_store = D1SessionStore::new(env.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(30)));

    routes::api_router()
        .layer(session_layer)
        .with_state(AppState::new(env))
}

#[event(fetch, respond_with_errors)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    Ok(router(env).call(req).await?)
}
