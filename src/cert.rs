use std::{fs::File, io::BufReader};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, PrivatePkcs1KeyDer, PrivateSec1KeyDer};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys, ec_private_keys};

pub fn load_certs(path: &str) -> std::io::Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(File::open(path)?);
    let certs = certs(&mut reader)?
        .into_iter()
        .map(|c| CertificateDer::from(c).into())
        .collect();
    Ok(certs)
}

pub fn load_key(path: &str) -> std::io::Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(File::open(path)?);
    if let Ok(keys) = pkcs8_private_keys(&mut reader) {
        if let Some(k) = keys.into_iter().next() {
            return Ok(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(k)).into());
        }
    }
    // Try SEC1 EC private key (commonly used by acme.sh for EC certificates)
    let mut reader = BufReader::new(File::open(path)?);
    if let Ok(keys) = ec_private_keys(&mut reader) {
        if let Some(k) = keys.into_iter().next() {
            return Ok(PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(k)).into());
        }
    }
    let mut reader = BufReader::new(File::open(path)?);
    if let Ok(keys) = rsa_private_keys(&mut reader) {
        if let Some(k) = keys.into_iter().next() {
            return Ok(PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(k)).into());
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid key"))
}
