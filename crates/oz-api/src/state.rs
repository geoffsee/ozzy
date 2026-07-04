use worker::send::SendWrapper;
use worker::Env;

use crate::crypto::decode_master_key;
use crate::error::{internal, AppResult};

#[derive(Clone)]
pub struct AppState {
    env: SendWrapper<Env>,
}

impl AppState {
    pub fn new(env: Env) -> Self {
        Self {
            env: SendWrapper::new(env),
        }
    }

    pub fn db(&self) -> AppResult<worker::D1Database> {
        self.env.d1("DB").map_err(|e| internal(e))
    }

    pub fn var(&self, name: &str) -> AppResult<String> {
        self.env
            .var(name)
            .map(|v| v.to_string())
            .map_err(|e| internal(e))
    }

    pub fn secret(&self, name: &str) -> AppResult<String> {
        self.env
            .secret(name)
            .map(|v| v.to_string())
            .map_err(|e| internal(e))
    }

    pub fn base_url(&self) -> AppResult<String> {
        self.var("OZ_BASE_URL")
    }

    pub fn test_mode(&self) -> bool {
        matches!(self.var("OZ_ENV").as_deref(), Ok("test"))
    }

    pub fn github_api_base(&self) -> AppResult<String> {
        self.var("GITHUB_API_BASE")
            .or_else(|_| Ok("https://github.com".into()))
    }

    pub fn master_key(&self) -> AppResult<Vec<u8>> {
        decode_master_key(&self.secret("OZ_MASTER_KEY")?)
    }

    pub fn api_key_pepper(&self) -> AppResult<String> {
        self.secret("OZ_API_KEY_PEPPER")
    }

    pub fn github_client_id(&self) -> AppResult<String> {
        self.secret("GITHUB_CLIENT_ID")
    }

    pub fn github_client_secret(&self) -> AppResult<String> {
        self.secret("GITHUB_CLIENT_SECRET")
    }
}
