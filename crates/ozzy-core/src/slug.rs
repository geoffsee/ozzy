const RESERVED: &[&str] = &["api", "auth", "v1", "test", "admin", "new", "static"];

pub fn validate_slug(slug: &str) -> Result<(), &'static str> {
    if slug.is_empty() {
        return Err("slug cannot be empty");
    }
    if slug.len() > 64 {
        return Err("slug too long");
    }
    if slug != slug.to_lowercase() {
        return Err("slug must be lowercase");
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("slug contains invalid characters");
    }
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err("slug cannot start or end with hyphen");
    }
    if slug.contains("--") {
        return Err("slug cannot contain consecutive hyphens");
    }
    if RESERVED.contains(&slug) {
        return Err("slug is reserved");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_slug() {
        assert!(validate_slug("my-project").is_ok());
        assert!(validate_slug("a1").is_ok());
    }

    #[test]
    fn rejects_invalid_slug() {
        assert!(validate_slug("").is_err());
        assert!(validate_slug("My-Project").is_err());
        assert!(validate_slug("my project").is_err());
        assert!(validate_slug("-bad").is_err());
        assert!(validate_slug("bad-").is_err());
        assert!(validate_slug("a--b").is_err());
        assert!(validate_slug("api").is_err());
    }
}
