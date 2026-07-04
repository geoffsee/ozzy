use oz_core::Profile;
use serde::Deserialize;
use worker::wasm_bindgen::JsValue;
use worker::D1Database;

use crate::db::d1_int;
use crate::error::{internal, AppResult};

#[derive(Deserialize)]
struct ProfileRow {
    id: String,
    github_id: i64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

impl From<ProfileRow> for Profile {
    fn from(r: ProfileRow) -> Self {
        Self {
            id: r.id,
            github_id: r.github_id,
            login: r.login,
            name: r.name,
            avatar_url: r.avatar_url,
        }
    }
}

pub async fn upsert_profile(
    db: &D1Database,
    github_id: i64,
    login: &str,
    name: Option<&str>,
    avatar_url: Option<&str>,
) -> AppResult<Profile> {
    let id = uuid::Uuid::new_v4().to_string();
    db.prepare(
        "INSERT INTO profiles (id, github_id, login, name, avatar_url)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(github_id) DO UPDATE SET
           login = excluded.login,
           name = excluded.name,
           avatar_url = excluded.avatar_url,
           updated_at = datetime('now')
         RETURNING id, github_id, login, name, avatar_url",
    )
    .bind(&[
        id.into(),
        d1_int(github_id),
        login.into(),
        name.map(|s| JsValue::from_str(s)).unwrap_or(JsValue::NULL),
        avatar_url
            .map(|s| JsValue::from_str(s))
            .unwrap_or(JsValue::NULL),
    ])?
    .first::<ProfileRow>(None)
    .await
    .map_err(|e| internal(e))?
    .map(Into::into)
    .ok_or_else(|| internal("profile upsert failed"))
}

pub async fn get_profile_by_id(db: &D1Database, id: &str) -> AppResult<Option<Profile>> {
    db.prepare("SELECT id, github_id, login, name, avatar_url FROM profiles WHERE id = ?1")
        .bind(&[id.into()])?
        .first::<ProfileRow>(None)
        .await
        .map_err(|e| internal(e))
        .map(|r| r.map(Into::into))
}

pub async fn get_profile_by_login(db: &D1Database, login: &str) -> AppResult<Option<Profile>> {
    db.prepare("SELECT id, github_id, login, name, avatar_url FROM profiles WHERE login = ?1")
        .bind(&[login.into()])?
        .first::<ProfileRow>(None)
        .await
        .map_err(|e| internal(e))
        .map(|r| r.map(Into::into))
}
