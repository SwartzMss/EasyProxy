use std::{env, sync::Arc};
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tokio_rustls::rustls::{self, ServerConfig};
use tokio_rustls::TlsAcceptor;
use log::info;

mod cert;
mod auth;
mod handler;

use cert::{load_certs, load_key};
use handler::handle_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    let cert = env::var("CERT").unwrap_or("cert.pem".into());
    let key = env::var("KEY").unwrap_or("key.pem".into());
    let user = env::var("USERNAME").unwrap_or("user".into());
    let pass = env::var("PASSWORD").unwrap_or("pass".into());
    let addr = env::var("ADDRESS").unwrap_or("0.0.0.0:8443".into());

    let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(load_certs(&cert)?, load_key(&key)?)?;
    config.alpn_protocols.push(b"http/1.1".to_vec());
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on https://{addr}");

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let user = user.clone();
        let pass = pass.clone();
        tokio::spawn(async move { handle_client(stream, acceptor, user, pass).await; });
    }
}
