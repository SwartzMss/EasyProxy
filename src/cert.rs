use std::{fs::File, io::BufReader};
use tokio_rustls::rustls::{Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};

pub fn load_certs(path: &str) -> std::io::Result<Vec<Certificate>> {
    let mut reader = BufReader::new(File::open(path)?);
    let certs = certs(&mut reader)?
        .into_iter()
        .map(Certificate)
        .collect();
    Ok(certs)
}

pub fn load_key(path: &str) -> std::io::Result<PrivateKey> {
    let mut reader = BufReader::new(File::open(path)?);
    if let Ok(keys) = pkcs8_private_keys(&mut reader) {
        if let Some(k) = keys.into_iter().next() {
            return Ok(PrivateKey(k));
        }
    }
    let mut reader = BufReader::new(File::open(path)?);
    if let Ok(keys) = rsa_private_keys(&mut reader) {
        if let Some(k) = keys.into_iter().next() {
            return Ok(PrivateKey(k));
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid key"))
}
