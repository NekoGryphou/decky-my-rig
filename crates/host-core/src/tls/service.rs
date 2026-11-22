use super::{types::FixedIdentity, utils::server_config_from_der};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use zeroize::Zeroizing;

impl FixedIdentity {
    pub fn from_der(certificates: Vec<Vec<u8>>, pkcs8_key: Vec<u8>) -> Result<Self, rustls::Error> {
        let mut pkcs8_key = Zeroizing::new(pkcs8_key);
        let leaf_der = certificates
            .first()
            .ok_or(rustls::Error::NoCertificatesPresented)?
            .clone();
        let digest = Sha256::digest(&leaf_der).into();
        let config = server_config_from_der(certificates, std::mem::take(&mut *pkcs8_key))?;
        Ok(Self {
            config,
            leaf_der,
            digest,
        })
    }
    pub fn leaf_der(&self) -> &[u8] {
        &self.leaf_der
    }
    pub fn config(&self) -> Arc<rustls::ServerConfig> {
        self.config.clone()
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}
