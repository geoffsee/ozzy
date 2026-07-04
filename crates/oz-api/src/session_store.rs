use std::fmt;

use async_trait::async_trait;
use time::OffsetDateTime;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{self, SessionStore};
use worker::send::SendWrapper;
use worker::Env;

pub const SESSION_PROFILE_KEY: &str = "profile_id";

#[derive(Clone)]
pub struct D1SessionStore {
    env: SendWrapper<Env>,
}

impl fmt::Debug for D1SessionStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("D1SessionStore").finish()
    }
}

impl D1SessionStore {
    pub fn new(env: Env) -> Self {
        Self {
            env: SendWrapper::new(env),
        }
    }

    fn db(&self) -> session_store::Result<worker::D1Database> {
        self.env
            .d1("DB")
            .map_err(|e| session_store::Error::Backend(e.to_string()))
    }
}

fn is_active(expiry_date: OffsetDateTime) -> bool {
    expiry_date > OffsetDateTime::now_utc()
}

fn encode_expiry(expiry: OffsetDateTime) -> session_store::Result<String> {
    expiry
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| session_store::Error::Encode(e.to_string()))
}

fn decode_expiry(value: &str) -> session_store::Result<OffsetDateTime> {
    OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map_err(|e| session_store::Error::Decode(e.to_string()))
}

#[async_trait]
impl SessionStore for D1SessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        create_record(self, record).await
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        save_record(self, record).await
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        load_record(self, session_id).await
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        delete_record(self, session_id).await
    }
}

#[worker::send]
async fn create_record(store: &D1SessionStore, record: &mut Record) -> session_store::Result<()> {
    loop {
        if load_record(store, &record.id).await?.is_none() {
            break;
        }
        record.id = Id::default();
    }
    save_record(store, record).await
}

#[worker::send]
async fn save_record(store: &D1SessionStore, record: &Record) -> session_store::Result<()> {
    let db = store.db()?;
    let data = serde_json::to_string(&record.data)
        .map_err(|e| session_store::Error::Encode(e.to_string()))?;
    let expiry = encode_expiry(record.expiry_date)?;

    db.prepare(
        "INSERT INTO tower_sessions (id, data, expiry_date) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET data = excluded.data, expiry_date = excluded.expiry_date",
    )
    .bind(&[
        record.id.to_string().into(),
        data.into(),
        expiry.into(),
    ])
    .map_err(|e| session_store::Error::Backend(e.to_string()))?
    .run()
    .await
    .map_err(|e| session_store::Error::Backend(e.to_string()))?;

    Ok(())
}

#[worker::send]
async fn load_record(store: &D1SessionStore, session_id: &Id) -> session_store::Result<Option<Record>> {
    let db = store.db()?;
    let row = db
        .prepare("SELECT data, expiry_date FROM tower_sessions WHERE id = ?1")
        .bind(&[session_id.to_string().into()])
        .map_err(|e| session_store::Error::Backend(e.to_string()))?
        .first::<serde_json::Value>(None)
        .await
        .map_err(|e| session_store::Error::Backend(e.to_string()))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let data_str = row["data"]
        .as_str()
        .ok_or_else(|| session_store::Error::Decode("missing data".into()))?;
    let expiry_str = row["expiry_date"]
        .as_str()
        .ok_or_else(|| session_store::Error::Decode("missing expiry".into()))?;

    let expiry_date = decode_expiry(expiry_str)?;
    if !is_active(expiry_date) {
        delete_record(store, session_id).await?;
        return Ok(None);
    }

    let data: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(data_str).map_err(|e| session_store::Error::Decode(e.to_string()))?;

    Ok(Some(Record {
        id: *session_id,
        data,
        expiry_date,
    }))
}

#[worker::send]
async fn delete_record(store: &D1SessionStore, session_id: &Id) -> session_store::Result<()> {
    let db = store.db()?;
    db.prepare("DELETE FROM tower_sessions WHERE id = ?1")
        .bind(&[session_id.to_string().into()])
        .map_err(|e| session_store::Error::Backend(e.to_string()))?
        .run()
        .await
        .map_err(|e| session_store::Error::Backend(e.to_string()))?;
    Ok(())
}
