use base64::Engine;
use oz_core::SecretMeta;
use serde::Deserialize;
use worker::D1Database;

use crate::db::d1_int;
use crate::error::{internal, AppResult};

#[derive(Deserialize)]
struct SecretRow {
    key_name: String,
    version: i64,
    updated_at: String,
    #[serde(default)]
    ciphertext: Option<Vec<u8>>,
    #[serde(default)]
    nonce: Option<Vec<u8>>,
}

pub async fn list_secrets(db: &D1Database, project_id: &str) -> AppResult<Vec<SecretMeta>> {
    let rows = db
        .prepare(
            "SELECT key_name, version, updated_at FROM secrets WHERE project_id = ?1 ORDER BY key_name",
        )
        .bind(&[project_id.into()])?
        .all()
        .await
        .map_err(|e| internal(e))?;

    rows.results::<SecretRow>()
        .map_err(|e| internal(e))?
        .into_iter()
        .map(|r| SecretMeta {
            key_name: r.key_name,
            version: r.version,
            updated_at: r.updated_at,
        })
        .collect::<Vec<_>>()
        .pipe(Ok)
}

pub async fn get_secret_row(
    db: &D1Database,
    project_id: &str,
    key_name: &str,
) -> AppResult<Option<(Vec<u8>, Vec<u8>, i64)>> {
    let row = db
        .prepare(
            "SELECT ciphertext, nonce, version FROM secrets
             WHERE project_id = ?1 AND key_name = ?2",
        )
        .bind(&[project_id.into(), key_name.into()])?
        .first::<serde_json::Value>(None)
        .await
        .map_err(|e| internal(e))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let ciphertext = decode_blob(&row["ciphertext"]).ok_or_else(|| internal("cipher"))?;
    let nonce = decode_blob(&row["nonce"]).ok_or_else(|| internal("nonce"))?;
    let version = row["version"].as_i64().unwrap_or(1);
    Ok(Some((ciphertext, nonce, version)))
}

pub async fn upsert_secret(
    db: &D1Database,
    project_id: &str,
    key_name: &str,
    ciphertext: &[u8],
    nonce: &[u8],
    profile_id: &str,
) -> AppResult<i64> {
    let existing = get_secret_row(db, project_id, key_name).await?;
    let version = existing.as_ref().map(|(_, _, v)| v + 1).unwrap_or(1);
    let id = uuid::Uuid::new_v4().to_string();

    if existing.is_some() {
        db.prepare(
            "UPDATE secrets SET ciphertext = ?1, nonce = ?2, version = ?3,
             updated_at = datetime('now'), updated_by_profile_id = ?4
             WHERE project_id = ?5 AND key_name = ?6",
        )
        .bind(&[
            ciphertext.to_vec().into(),
            nonce.to_vec().into(),
            d1_int(version),
            profile_id.into(),
            project_id.into(),
            key_name.into(),
        ])?
        .run()
        .await
        .map_err(|e| internal(e))?;
    } else {
        db.prepare(
            "INSERT INTO secrets (id, project_id, key_name, ciphertext, nonce, version, updated_by_profile_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&[
            id.into(),
            project_id.into(),
            key_name.into(),
            ciphertext.to_vec().into(),
            nonce.to_vec().into(),
            d1_int(version),
            profile_id.into(),
        ])?
        .run()
        .await
        .map_err(|e| internal(e))?;
    }

    Ok(version)
}

pub async fn delete_secret(db: &D1Database, project_id: &str, key_name: &str) -> AppResult<bool> {
    let result = db
        .prepare("DELETE FROM secrets WHERE project_id = ?1 AND key_name = ?2")
        .bind(&[project_id.into(), key_name.into()])?
        .run()
        .await
        .map_err(|e| internal(e))?;
    Ok(result.meta()?.and_then(|m| m.changes).unwrap_or(0) > 0)
}

fn decode_blob(v: &serde_json::Value) -> Option<Vec<u8>> {
    if let Some(s) = v.as_str() {
        return base64::engine::general_purpose::STANDARD.decode(s).ok();
    }
    v.as_array().map(|arr| {
        arr.iter()
            .filter_map(|n| n.as_u64().map(|x| x as u8))
            .collect()
    })
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
