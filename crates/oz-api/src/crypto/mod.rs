use aes_gcm::aead::{Aead, KeyInit as AesKeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::error::{internal, AppResult};

type HmacSha256 = Hmac<Sha256>;

const GCM_IV_LEN: usize = 12;
const DEK_LEN: usize = 32;

pub fn random_bytes(len: usize) -> AppResult<Vec<u8>> {
    let mut buf = vec![0u8; len];
    getrandom::fill(&mut buf).map_err(|_| internal("rng failure"))?;
    Ok(buf)
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex_encode(&Sha256::digest(data))
}

pub fn sha256_raw(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

pub fn hmac_sha256_b64(key_material: &[u8], data: &[u8]) -> AppResult<String> {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(key_material).map_err(|_| internal("hmac key"))?;
    mac.update(data);
    Ok(base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn decode_master_key(b64: &str) -> AppResult<Vec<u8>> {
    let key = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| internal("master key decode"))?;
    if key.len() != DEK_LEN {
        return Err(internal("master key length"));
    }
    Ok(key)
}

pub struct WrappedDek {
    pub wrapped: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub fn generate_and_wrap_dek(master_key: &[u8]) -> AppResult<(Vec<u8>, WrappedDek)> {
    let dek = random_bytes(DEK_LEN)?;
    let wrapped = wrap_dek(master_key, &dek)?;
    Ok((dek, wrapped))
}

pub fn wrap_dek(master_key: &[u8], dek: &[u8]) -> AppResult<WrappedDek> {
    let nonce = random_bytes(GCM_IV_LEN)?;
    let cipher = Aes256Gcm::new_from_slice(master_key).map_err(|_| internal("master key"))?;
    let wrapped = cipher
        .encrypt(Nonce::from_slice(&nonce), dek)
        .map_err(|_| internal("wrap dek"))?;
    Ok(WrappedDek { wrapped, nonce })
}

pub fn unwrap_dek(master_key: &[u8], wrapped: &[u8], nonce: &[u8]) -> AppResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(master_key).map_err(|_| internal("master key"))?;
    cipher
        .decrypt(Nonce::from_slice(nonce), wrapped)
        .map_err(|_| internal("unwrap dek"))
}

pub fn encrypt_secret(dek: &[u8], plaintext: &str) -> AppResult<(Vec<u8>, Vec<u8>)> {
    let nonce = random_bytes(GCM_IV_LEN)?;
    let cipher = Aes256Gcm::new_from_slice(dek).map_err(|_| internal("dek"))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|_| internal("encrypt"))?;
    Ok((ciphertext, nonce))
}

pub fn decrypt_secret(dek: &[u8], ciphertext: &[u8], nonce: &[u8]) -> AppResult<String> {
    let cipher = Aes256Gcm::new_from_slice(dek).map_err(|_| internal("dek"))?;
    let plain = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| internal("decrypt"))?;
    String::from_utf8(plain).map_err(|_| internal("utf8"))
}

pub fn generate_api_key_token() -> String {
    format!(
        "{}{}",
        oz_core::API_KEY_PREFIX,
        uuid::Uuid::new_v4().simple()
    )
}
