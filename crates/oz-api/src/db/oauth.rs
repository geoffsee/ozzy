use worker::D1Database;

use crate::error::{internal, AppResult};

pub async fn create_oauth_state(
    db: &D1Database,
    state: &str,
    code_verifier: &str,
    expires_at: &str,
) -> AppResult<()> {
    db.prepare("INSERT INTO oauth_states (state, code_verifier, expires_at) VALUES (?1, ?2, ?3)")
        .bind(&[state.into(), code_verifier.into(), expires_at.into()])?
        .run()
        .await
        .map_err(|e| internal(e))?;
    Ok(())
}

pub async fn consume_oauth_state(db: &D1Database, state: &str) -> AppResult<Option<String>> {
    let row = db
        .prepare("SELECT code_verifier, expires_at FROM oauth_states WHERE state = ?1")
        .bind(&[state.into()])?
        .first::<serde_json::Value>(None)
        .await
        .map_err(|e| internal(e))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let verifier = row["code_verifier"]
        .as_str()
        .ok_or_else(|| internal("verifier"))?
        .to_string();
    let expires_at = row["expires_at"]
        .as_str()
        .ok_or_else(|| internal("expires"))?;

    db.prepare("DELETE FROM oauth_states WHERE state = ?1")
        .bind(&[state.into()])?
        .run()
        .await
        .map_err(|e| internal(e))?;

    let now = time::OffsetDateTime::now_utc();
    let exp =
        time::OffsetDateTime::parse(expires_at, &time::format_description::well_known::Rfc3339)
            .map_err(|_| internal("parse exp"))?;
    if now >= exp {
        return Ok(None);
    }

    Ok(Some(verifier))
}

pub async fn cleanup_expired_oauth_states(db: &D1Database) -> AppResult<()> {
    db.prepare("DELETE FROM oauth_states WHERE expires_at < datetime('now')")
        .run()
        .await
        .map_err(|e| internal(e))?;
    Ok(())
}
