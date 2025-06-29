use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};
use tokio_rustls::TlsAcceptor;
use log::{info, error, warn};
use crate::auth::verify_basic_auth;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;

// 获取系统代理设置
fn get_system_proxy() -> Option<(String, u16)> {
    // 从环境变量或配置文件读取代理设置
    let proxy_urls = [
        env::var("HTTP_PROXY").ok(),
        env::var("HTTPS_PROXY").ok(),
        env::var("http_proxy").ok(),
        env::var("https_proxy").ok(),
    ];
    
    for proxy in proxy_urls {
        if let Some(proxy_url) = proxy {
            info!("检测到代理设置: {}", proxy_url);
            if let Some((host, port)) = parse_proxy_url(&proxy_url) {
                info!("使用代理: {}:{}", host, port);
                return Some((host, port));
            }
        }
    }
    
    info!("未找到代理设置，将直接连接目标");
    None
}

// 解析代理 URL
fn parse_proxy_url(url: &str) -> Option<(String, u16)> {
    let url = url.trim();
    if url.starts_with("http://") {
        let host_port = &url[7..];
        if let Some(colon_pos) = host_port.rfind(':') {
            let host = host_port[..colon_pos].to_string();
            if let Ok(port) = host_port[colon_pos + 1..].parse::<u16>() {
                return Some((host, port));
            }
        }
    }
    None
}

fn record_connection(ip: std::net::IpAddr, target: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    match OpenOptions::new().create(true).append(true).open("connections.txt") {
        Ok(mut file) => {
            let line = format!("{} {} {}\n", now, ip, target);
            if let Err(e) = file.write_all(line.as_bytes()) {
                error!("写入连接记录失败: {}", e);
            }
        }
        Err(e) => {
            error!("无法打开连接记录文件: {}", e);
        }
    }
}

// 通过代理连接目标
async fn connect_through_proxy(proxy_host: &str, proxy_port: u16, target: &str) -> Result<TcpStream, Box<dyn std::error::Error + Send + Sync>> {
    info!("通过代理 {}:{} 连接目标: {}", proxy_host, proxy_port, target);
    
    // 连接到代理服务器
    let mut proxy_stream = TcpStream::connect(format!("{}:{}", proxy_host, proxy_port)).await?;
    
    // 发送 CONNECT 请求到代理
    let connect_request = format!("CONNECT {} HTTP/1.1\r\nHost: {}\r\n\r\n", target, target);
    proxy_stream.write_all(connect_request.as_bytes()).await?;
    
    // 读取代理响应
    let mut response = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        let n = proxy_stream.read(&mut buffer).await?;
        if n == 0 { break; }
        response.extend_from_slice(&buffer[..n]);
        if response.ends_with(b"\r\n\r\n") { break; }
    }
    
    let response_str = String::from_utf8_lossy(&response);
    info!("代理响应: {}", response_str.lines().next().unwrap_or(""));
    
    // 检查响应是否成功
    if response_str.contains("200") {
        info!("代理连接成功");
        Ok(proxy_stream)
    } else {
        Err(format!("代理连接失败: {}", response_str).into())
    }
}

pub async fn handle_client(
    stream: TcpStream,
    acceptor: TlsAcceptor,
    username: String,
    password: String,
) {
    let peer = match stream.peer_addr() { Ok(a) => a, Err(_) => return };
    info!("新的连接来自: {peer}");
    
    // TLS 握手
    info!("开始 TLS 握手...");
    let mut stream = match acceptor.accept(stream).await {
        Ok(s) => {
            info!("TLS 握手成功，连接已建立: {peer}");
            s
        },
        Err(e) => { 
            error!("TLS 握手失败 {peer}: {e}"); 
            return; 
        }
    };
    
    // 读取 HTTP 请求头
    info!("开始读取 HTTP 请求头...");
    let mut buf = Vec::new();
    let mut header_size = 0;
    loop {
        let mut byte = [0u8;1];
        match stream.read(&mut byte).await {
            Ok(0) => {
                warn!("客户端 {peer} 在读取请求头时断开连接");
                return;
            },
            Ok(_) => {
                buf.push(byte[0]);
                header_size += 1;
                if buf.ends_with(b"\r\n\r\n") { 
                    info!("请求头读取完成，大小: {} 字节", header_size);
                    break; 
                }
                if buf.len() > 8192 { 
                    error!("请求头过大 {peer}: {} 字节", buf.len()); 
                    return; 
                }
            },
            Err(e) => { 
                error!("读取请求头错误 {peer}: {e}"); 
                return; 
            }
        }
    }
    
    // 解析 HTTP 请求
    let req = match String::from_utf8(buf) { 
        Ok(s) => {
            info!("HTTP 请求内容:\n{}", s);
            s
        }, 
        Err(e) => {
            error!("HTTP 请求解析失败 {peer}: {e}");
            return; 
        } 
    };
    
    let mut lines = req.lines();
    let first = lines.next().unwrap_or("");
    info!("请求行: {}", first);
    
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    
    info!("请求方法: {}, 目标: {}", method, target);
    
    if method != "CONNECT" {
        warn!("不支持的请求方法 {peer}: {}", method);
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n").await;
        return;
    }
    
    // 认证检查
    info!("开始认证检查...");
    let mut auth_ok = false;
    let mut auth_header_found = false;
    
    info!("=== 完整的 HTTP 请求头 ===");
    for line in lines {
        if line.is_empty() { 
            info!("=== 请求头结束 ===");
            break; 
        }
        info!("请求头: {}", line);
        
        if line.to_ascii_lowercase().starts_with("proxy-authorization:") {
            auth_header_found = true;
            info!("找到认证头: {}", line);
            if let Some(value) = line.splitn(2, ':').nth(1) {
                let value = value.trim();
                info!("认证值: {}", value);
                if verify_basic_auth(value, &username, &password) {
                    auth_ok = true;
                    info!("认证成功 {peer}");
                } else {
                    warn!("认证失败 {peer}: 用户名或密码错误");
                }
            }
        }
    }
    
    if !auth_header_found {
        warn!("未找到认证头 {peer} - 请求中没有 Proxy-Authorization 字段");
        info!("请检查 SwitchyOmega 配置是否正确设置了用户名和密码");
    }
    
    if !auth_ok {
        if !auth_header_found {
            info!("认证失败 {peer}: 缺少认证头");
        } else {
            info!("认证失败 {peer}: 用户名或密码错误");
        }
        let _ = stream
            .write_all(
                b"HTTP/1.1 407 Proxy Authentication Required\r\n\
Proxy-Authenticate: Basic realm=\"EasyProxy\"\r\n\r\n",
            )
            .await;
        return;
    }

    record_connection(peer.ip(), target);
    
    // 连接目标服务器（通过系统代理或直接连接）
    info!("开始连接目标服务器: {}", target);
    let mut remote = match get_system_proxy() {
        Some((proxy_host, proxy_port)) => {
            match connect_through_proxy(&proxy_host, proxy_port, target).await {
                Ok(stream) => {
                    info!("通过系统代理成功连接到目标服务器: {}", target);
                    stream
                },
                Err(e) => {
                    error!("通过系统代理连接失败 {}: {}", target, e);
                    let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                    return;
                }
            }
        },
        None => {
            match TcpStream::connect(target).await {
                Ok(s) => {
                    info!("直接连接成功连接到目标服务器: {}", target);
                    s
                },
                Err(e) => {
                    error!("直接连接目标服务器失败 {}: {}", target, e);
                    let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                    return;
                }
            }
        }
    };
    
    // 发送连接成功响应
    info!("发送 200 Connection Established 响应");
    if stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
        .is_err()
    {
        error!("发送连接成功响应失败 {peer}");
        return;
    }
    
    info!("开始双向数据转发 {peer} <-> {}", target);
    let result = tokio::io::copy_bidirectional(&mut stream, &mut remote).await;
    match result {
        Ok((bytes_from_client, bytes_to_client)) => {
            info!("连接正常结束 {peer} <-> {}: 客户端→服务器 {} 字节, 服务器→客户端 {} 字节", 
                  target, bytes_from_client, bytes_to_client);
        },
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("close_notify") {
                warn!("客户端未发送 TLS close_notify {peer} <-> {}: {}", target, error_msg);
            } else {
                error!("连接异常结束 {peer} <-> {}: {}", target, e);
            }
        }
    }
}
