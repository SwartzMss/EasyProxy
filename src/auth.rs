use base64::Engine;
use log::info;

pub fn verify_basic_auth(value: &str, username: &str, password: &str) -> bool {
    info!("开始验证 Basic Auth: {}", value);
    info!("期望的用户名: {}, 期望的密码: {}", username, password);
    
    if let Some(b64) = value.strip_prefix("Basic ") {
        info!("提取 Base64 编码: {}", b64);
        
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64) {
            info!("Base64 解码成功，长度: {} 字节", decoded.len());
            
            if let Ok(s) = String::from_utf8(decoded) {
                info!("解码后的认证信息: {}", s);
                let expected = format!("{}:{}", username, password);
                info!("期望的认证信息: {}", expected);
                
                let result = s == expected;
                info!("认证结果: {}", if result { "成功" } else { "失败" });
                return result;
            } else {
                info!("Base64 解码后不是有效的 UTF-8 字符串");
            }
        } else {
            info!("Base64 解码失败");
        }
    } else {
        info!("不是 Basic 认证格式");
    }
    
    info!("认证失败");
    false
}
