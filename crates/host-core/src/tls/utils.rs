use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::Arc;
use zeroize::Zeroizing;

/// The caller provides and checks the certificate and key files in protected
/// storage. Only DER certificates and a DER PKCS#8 private key are accepted.
pub(super) fn server_config_from_der(
    certificates: Vec<Vec<u8>>,
    pkcs8_key: Vec<u8>,
) -> Result<Arc<rustls::ServerConfig>, rustls::Error> {
    // ! rustls-pki-types supports Zeroize, but its owned DER wrapper doesn't wipe
    //   itself on drop. Keep this copy guarded while ring borrows it.
    let key = Zeroizing::new(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8_key)));
    let signing_key = rustls::crypto::ring::sign::any_supported_type(&key)?;
    let certified_key = rustls::sign::CertifiedKey::new(
        certificates.into_iter().map(CertificateDer::from).collect(),
        signing_key,
    );
    certified_key.keys_match()?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])?
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(rustls::sign::SingleCertAndKey::from(
            certified_key,
        )));
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config.max_early_data_size = 0;
    Ok(Arc::new(config))
}
