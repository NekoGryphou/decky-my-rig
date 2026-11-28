use super::{consts::MAX_CERTIFICATE_BYTES, types::PinVerifier};
use rustls::{
    CertificateError, DigitallySignedStruct, Error, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex, PoisonError};
use subtle::ConstantTimeEq;

impl PinVerifier {
    pub(in crate::host_client) fn new(
        pin: Option<[u8; 32]>,
        provider: Arc<CryptoProvider>,
    ) -> Self {
        Self {
            pin,
            provider,
            observed: Mutex::new(None),
        }
    }

    /// The certificate the peer presented, if one within the size limit was seen.
    pub(in crate::host_client) fn certificate(&self) -> Option<Vec<u8>> {
        self.observed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// True when we saw a certificate and it isn't the pinned one.
    pub(in crate::host_client) fn mismatched(&self) -> bool {
        self.certificate()
            .is_some_and(|certificate| !self.matches(&certificate))
    }

    fn matches(&self, certificate: &[u8]) -> bool {
        self.pin
            .is_none_or(|pin| bool::from(Sha256::digest(certificate).as_slice().ct_eq(&pin[..])))
    }
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if end_entity.len() > MAX_CERTIFICATE_BYTES {
            return Err(Error::InvalidCertificate(CertificateError::BadEncoding));
        }
        *self.observed.lock().unwrap_or_else(PoisonError::into_inner) = Some(end_entity.to_vec());
        if !self.matches(end_entity) {
            return Err(Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ));
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
