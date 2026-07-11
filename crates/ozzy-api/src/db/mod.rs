pub mod api_keys;
pub mod oauth;
pub mod profiles;
pub mod projects;
pub mod secrets;

pub use projects::ProjectCryptoRow;

use worker::wasm_bindgen::JsValue;
use worker::D1Database;

use crate::error::{internal, AppResult};

/// D1 bindings reject JS bigint; bind SQLite integers as strings.
pub fn d1_int(n: i64) -> JsValue {
    JsValue::from_str(&n.to_string())
}

pub async fn exec(db: &D1Database, sql: &str) -> AppResult<()> {
    db.exec(sql).await.map_err(|e| internal(e))?;
    Ok(())
}
