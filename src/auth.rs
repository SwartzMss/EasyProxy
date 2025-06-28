use base64::Engine;

pub fn verify_basic_auth(value: &str, username: &str, password: &str) -> bool {
    const PREFIX: &str = "Basic ";
    if value.len() > PREFIX.len() && value[..PREFIX.len()].eq_ignore_ascii_case(PREFIX) {
        let b64 = &value[PREFIX.len()..];
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64) {
            if let Ok(s) = String::from_utf8(decoded) {
                return s == format!("{}:{}", username, password);
            }
        }
    }
    false
}
