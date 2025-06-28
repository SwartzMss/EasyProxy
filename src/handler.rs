use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};
use tokio_rustls::TlsAcceptor;
use log::{info, error};
use crate::auth::verify_basic_auth;

pub async fn handle_client(
    stream: TcpStream,
    acceptor: TlsAcceptor,
    username: String,
    password: String,
) {
    let peer = match stream.peer_addr() { Ok(a) => a, Err(_) => return };
    info!("Connection from {peer}");
    let mut stream = match acceptor.accept(stream).await {
        Ok(s) => s,
        Err(e) => { error!("TLS error from {peer}: {e}"); return; }
    };
    info!("TLS established with {peer}");
    // read headers
    let mut buf = Vec::new();
    loop {
        let mut byte = [0u8;1];
        match stream.read(&mut byte).await {
            Ok(0) => return,
            Ok(_) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") { break; }
                if buf.len() > 8192 { error!("header too large from {peer}"); return; }
            },
            Err(e) => { error!("read error from {peer}: {e}"); return; }
        }
    }
    let req = match String::from_utf8(buf) { Ok(s) => s, Err(_) => return };
    let mut lines = req.lines();
    let first = lines.next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    if method != "CONNECT" {
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n").await;
        return;
    }
    let mut auth_ok = false;
    for line in lines {
        if line.is_empty() { break; }
        if line.to_ascii_lowercase().starts_with("proxy-authorization:") {
            if let Some(value) = line.splitn(2, ':').nth(1) {
                let value = value.trim();
                if verify_basic_auth(value, &username, &password) {
                    auth_ok = true;
                }
            }
        }
    }
    if !auth_ok {
        let _ = stream.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n").await;
        info!("Authentication failed from {peer}");
        return;
    }
    info!("{peer} authenticated, connecting to {target}");
    let mut remote = match TcpStream::connect(target).await {
        Ok(s) => s,
        Err(e) => {
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            error!("Failed to connect to {target}: {e}");
            return;
        }
    };
    if stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
        .is_err()
    {
        return;
    }
    let _ = tokio::io::copy_bidirectional(&mut stream, &mut remote).await;
    info!("Connection with {peer} closed");
}
