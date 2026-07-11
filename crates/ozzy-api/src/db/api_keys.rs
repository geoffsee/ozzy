use ozzy_core::{ApiKeyInfo, ApiKeyPermission, ApiKeyScope, CreateApiKeyResponse};
use serde::Deserialize;
use worker::D1Database;

use crate::crypto::{constant_time_eq, generate_api_key_token, hmac_sha256_b64};
use crate::error::{internal, AppResult};
use ozzy_core::api_key_prefix;

#[derive(Clone, Debug)]
pub struct ResolvedApiKey {
    pub id: String,
    pub profile_id: String,
    pub scopes: Vec<ApiKeyScope>,
}

pub async fn create_api_key(
    db: &D1Database,
    profile_id: &str,
    name: &str,
    scopes: &[(String, ApiKeyPermission)],
    pepper: &str,
) -> AppResult<CreateApiKeyResponse> {
    let raw_key = generate_api_key_token();
    let key_hash = hmac_sha256_b64(pepper.as_bytes(), raw_key.as_bytes())?;
    let prefix = api_key_prefix(&raw_key);
    let id = uuid::Uuid::new_v4().to_string();

    db.prepare(
        "INSERT INTO api_keys (id, profile_id, name, key_prefix, key_hash)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&[
        id.clone().into(),
        profile_id.into(),
        name.into(),
        prefix.clone().into(),
        key_hash.into(),
    ])?
    .run()
    .await
    .map_err(|e| internal(e))?;

    let mut scope_objs = Vec::new();
    for (project_id, permission) in scopes {
        db.prepare(
            "INSERT INTO api_key_scopes (api_key_id, project_id, permission)
             VALUES (?1, ?2, ?3)",
        )
        .bind(&[
            id.clone().into(),
            project_id.clone().into(),
            permission.as_str().into(),
        ])?
        .run()
        .await
        .map_err(|e| internal(e))?;
        scope_objs.push(ApiKeyScope {
            project_id: project_id.clone(),
            permission: *permission,
        });
    }

    Ok(CreateApiKeyResponse {
        id,
        name: name.to_string(),
        key: raw_key,
        key_prefix: prefix,
        scopes: scope_objs,
    })
}

pub async fn list_api_keys(db: &D1Database, profile_id: &str) -> AppResult<Vec<ApiKeyInfo>> {
    let rows = db
        .prepare(
            "SELECT id, name, key_prefix FROM api_keys
             WHERE profile_id = ?1 AND revoked_at IS NULL
             ORDER BY created_at DESC",
        )
        .bind(&[profile_id.into()])?
        .all()
        .await
        .map_err(|e| internal(e))?;

    let keys = rows
        .results::<serde_json::Value>()
        .map_err(|e| internal(e))?;
    let mut out = Vec::new();
    for row in keys {
        let id = row["id"].as_str().unwrap_or_default().to_string();
        let scopes = load_scopes(db, &id).await?;
        out.push(ApiKeyInfo {
            id,
            name: row["name"].as_str().unwrap_or_default().to_string(),
            key_prefix: row["key_prefix"].as_str().unwrap_or_default().to_string(),
            scopes,
        });
    }
    Ok(out)
}

async fn load_scopes(db: &D1Database, api_key_id: &str) -> AppResult<Vec<ApiKeyScope>> {
    let rows = db
        .prepare("SELECT project_id, permission FROM api_key_scopes WHERE api_key_id = ?1")
        .bind(&[api_key_id.into()])?
        .all()
        .await
        .map_err(|e| internal(e))?;

    rows.results::<serde_json::Value>()
        .map_err(|e| internal(e))?
        .into_iter()
        .filter_map(|r| {
            Some(ApiKeyScope {
                project_id: r["project_id"].as_str()?.to_string(),
                permission: ApiKeyPermission::parse(r["permission"].as_str()?)?,
            })
        })
        .collect::<Vec<_>>()
        .pipe(Ok)
}

pub async fn revoke_api_key(db: &D1Database, profile_id: &str, key_id: &str) -> AppResult<bool> {
    let result = db
        .prepare(
            "UPDATE api_keys SET revoked_at = datetime('now')
             WHERE id = ?1 AND profile_id = ?2 AND revoked_at IS NULL",
        )
        .bind(&[key_id.into(), profile_id.into()])?
        .run()
        .await
        .map_err(|e| internal(e))?;
    Ok(result.meta()?.and_then(|m| m.changes).unwrap_or(0) > 0)
}

pub async fn resolve_api_key(
    db: &D1Database,
    raw_key: &str,
    pepper: &str,
) -> AppResult<Option<ResolvedApiKey>> {
    let key_hash = hmac_sha256_b64(pepper.as_bytes(), raw_key.as_bytes())?;
    let row = db
        .prepare(
            "SELECT id, profile_id, key_hash, expires_at, revoked_at FROM api_keys WHERE key_hash = ?1",
        )
        .bind(&[key_hash.clone().into()])?
        .first::<serde_json::Value>(None)
        .await
        .map_err(|e| internal(e))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let stored_hash = row["key_hash"].as_str().unwrap_or_default();
    if !constant_time_eq(stored_hash, &key_hash) {
        return Ok(None);
    }

    if row["revoked_at"].as_str().is_some() {
        return Ok(None);
    }

    if let Some(exp) = row["expires_at"].as_str() {
        let exp = time::OffsetDateTime::parse(exp, &time::format_description::well_known::Rfc3339)
            .map_err(|_| internal("exp"))?;
        if time::OffsetDateTime::now_utc() >= exp {
            return Ok(None);
        }
    }

    let id = row["id"].as_str().unwrap_or_default().to_string();
    let profile_id = row["profile_id"].as_str().unwrap_or_default().to_string();
    let scopes = load_scopes(db, &id).await?;

    db.prepare("UPDATE api_keys SET last_used_at = datetime('now') WHERE id = ?1")
        .bind(&[id.clone().into()])?
        .run()
        .await
        .map_err(|e| internal(e))?;

    Ok(Some(ResolvedApiKey {
        id,
        profile_id,
        scopes,
    }))
}

pub async fn create_expired_api_key_for_test(
    db: &D1Database,
    profile_id: &str,
    pepper: &str,
) -> AppResult<String> {
    let raw_key = generate_api_key_token();
    let key_hash = hmac_sha256_b64(pepper.as_bytes(), raw_key.as_bytes())?;
    let id = uuid::Uuid::new_v4().to_string();
    let expires_at = (time::OffsetDateTime::now_utc() - time::Duration::hours(1))
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| internal("time"))?;

    db.prepare(
        "INSERT INTO api_keys (id, profile_id, name, key_prefix, key_hash, expires_at)
         VALUES (?1, ?2, 'expired', 'ozzy_live_expired', ?3, ?4)",
    )
    .bind(&[
        id.into(),
        profile_id.into(),
        key_hash.into(),
        expires_at.into(),
    ])?
    .run()
    .await
    .map_err(|e| internal(e))?;

    Ok(raw_key)
}

trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ApiKeyRow {
    id: String,
    profile_id: String,
    key_hash: String,
}
