use base64;

pub fn verify_basic_auth(value: &str, username: &str, password: &str) -> bool {
    if let Some(b64) = value.strip_prefix("Basic ") {
        if let Ok(decoded) = base64::decode(b64) {
            if let Ok(s) = String::from_utf8(decoded) {
                return s == format!("{}:{}", username, password);
            }
        }
    }
    false
}
