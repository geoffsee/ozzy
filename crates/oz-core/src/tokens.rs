pub const API_KEY_PREFIX: &str = "oz_live_";

pub fn parse_bearer(header: Option<&str>) -> Option<&str> {
    let header = header?;
    let prefix = "Bearer ";
    header.strip_prefix(prefix).map(str::trim).filter(|t| !t.is_empty())
}

pub fn parse_api_key(raw: &str) -> Option<&str> {
    raw.strip_prefix(API_KEY_PREFIX)
        .filter(|k| k.len() >= 32)
}

pub fn api_key_prefix(raw: &str) -> String {
    raw.chars().take(16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bearer_header() {
        assert_eq!(
            parse_bearer(Some("Bearer oz_live_abc")),
            Some("oz_live_abc")
        );
        assert_eq!(parse_bearer(Some("Basic x")), None);
        assert_eq!(parse_bearer(None), None);
    }

    #[test]
    fn parse_api_key_format() {
        let key = format!("{API_KEY_PREFIX}{}", "a".repeat(32));
        assert!(parse_api_key(&key).is_some());
        assert!(parse_api_key("oz_live_short").is_none());
    }
}
