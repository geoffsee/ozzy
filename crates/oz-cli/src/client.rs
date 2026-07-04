use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;

use crate::config::Config;

pub struct ApiClient {
    client: Client,
    base: String,
    api_key: String,
}

impl ApiClient {
    pub fn from_env() -> Result<Self> {
        let mut cfg = Config::load()?;
        if let Ok(url) = std::env::var("OZ_API_URL") {
            cfg.api_url = url;
        }
        if let Ok(key) = std::env::var("OZ_API_KEY") {
            cfg.api_key = Some(key);
        }
        let api_key = cfg.api_key.context("not logged in; run `oz auth login`")?;
        Ok(Self {
            client: Client::new(),
            base: cfg.api_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self
            .client
            .get(format!("{}{}", self.base, path))
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .send()
            .context("request failed")?;
        self.parse(resp)
    }

    pub fn put<T: DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> Result<T> {
        let resp = self
            .client
            .put(format!("{}{}", self.base, path))
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .context("request failed")?;
        self.parse(resp)
    }

    pub fn delete(&self, path: &str) -> Result<()> {
        let resp = self
            .client
            .delete(format!("{}{}", self.base, path))
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .send()
            .context("request failed")?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("API error {status}: {body}");
        }
    }

    fn parse<T: DeserializeOwned>(&self, resp: reqwest::blocking::Response) -> Result<T> {
        if resp.status().is_success() {
            Ok(resp.json()?)
        } else {
            let status = resp.status();
            let err: serde_json::Value = resp.json().unwrap_or_default();
            let msg = err
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("request failed");
            bail!("API error {status}: {msg}");
        }
    }
}
