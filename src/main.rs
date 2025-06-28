use std::{env, sync::Arc};
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use log::{info, error, warn};

mod logger;

mod cert;
mod auth;
mod handler;

use cert::{load_certs, load_key};
use handler::handle_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init()?;
    info!("启动 EasyProxy");
    match dotenv() {
        Ok(_) => info!("已加载 .env 文件"),
        Err(e) => {
            error!("未找到 .env 文件: {}", e);
            std::process::exit(1);
        },
    }

    let cert = env::var("CERT").unwrap_or_else(|_| {
        warn!("未设置 CERT 环境变量，使用默认值 cert.pem");
        "cert.pem".into()
    });
    let key = env::var("KEY").unwrap_or_else(|_| {
        warn!("未设置 KEY 环境变量，使用默认值 key.pem");
        "key.pem".into()
    });
    let user = env::var("USER").unwrap_or_else(|_| {
        warn!("未设置 USER 环境变量，使用默认值 user");
        "user".into()
    });
    let pass = env::var("PASSWD").unwrap_or_else(|_| {
        warn!("未设置 PASSWD 环境变量，使用默认值 pass");
        "pass".into()
    });
    let addr = env::var("ADDRESS").unwrap_or_else(|_| {
        warn!("未设置 ADDRESS 环境变量，使用默认值 0.0.0.0:8443");
        "0.0.0.0:8443".into()
    });

    info!("加载证书路径: {}", cert);
    info!("加载密钥路径: {}", key);
    info!("用户名: {}", user);
    info!("密码: {}", pass);
    info!("地址: {}", addr);

    let certs = match load_certs(&cert) {
        Ok(c) => {
            info!("证书加载成功，共 {} 条", c.len());
            c
        },
        Err(e) => {
            error!("加载证书失败: {}", e);
            return Err(e.into());
        }
    };
    let key = match load_key(&key) {
        Ok(k) => {
            info!("密钥加载成功");
            k
        },
        Err(e) => {
            error!("加载密钥失败: {}", e);
            return Err(e.into());
        }
    };

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    config.alpn_protocols.push(b"http/1.1".to_vec());
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(&addr).await?;
    info!("监听地址: https://{addr}");
    info!("代理已启动，等待连接...");

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let user = user.clone();
        let pass = pass.clone();
        tokio::spawn(async move { handle_client(stream, acceptor, user, pass).await; });
    }
}
