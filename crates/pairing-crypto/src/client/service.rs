use super::{errors::ClientError, types::ClientExchange};
use crate::{
    CONFIRMATION_LEN, MAX_CERTIFICATE_BYTES, REQUEST_LEN, Suite, context, decode_response,
    identities, stretch,
};
use opaque_ke::{ClientLogin, ClientLoginFinishParameters};
use rand::{
    SeedableRng,
    rngs::{StdRng, SysRng},
};
use zeroize::Zeroizing;

impl ClientExchange {
    /// Starts an exchange for the eight-digit `code`, tied to the host's TLS
    /// certificate (taken from the connection, never from a message) and the
    /// host's session nonce. Returns the request to send.
    pub fn start(
        code: &[u8],
        certificate_der: &[u8],
        session: &[u8; 32],
    ) -> Result<(Self, [u8; REQUEST_LEN]), ClientError> {
        if code.len() != 8
            || !code.iter().all(u8::is_ascii_digit)
            || certificate_der.is_empty()
            || certificate_der.len() > MAX_CERTIFICATE_BYTES
        {
            return Err(ClientError::InvalidInput);
        }
        let start = ClientLogin::<Suite>::start(&mut seeded_rng()?, code)
            .map_err(|_| ClientError::Internal)?;
        let request = start
            .message
            .serialize()
            .as_slice()
            .try_into()
            .map_err(|_| ClientError::Internal)?;
        Ok((
            Self {
                state: start.state,
                code: Zeroizing::new(code.to_vec()),
                context: context(certificate_der, session),
            },
            request,
        ))
    }

    /// Authenticates the host from its response and returns the confirmation to
    /// send. Success proves who the host is, not that it saved a credential.
    pub fn finish(self, response: &[u8]) -> Result<[u8; CONFIRMATION_LEN], ClientError> {
        let response = decode_response(response).map_err(|()| {
            if response.len() == crate::RESPONSE_LEN {
                ClientError::AuthenticationFailed
            } else {
                ClientError::InvalidInput
            }
        })?;
        let ksf = stretch();
        let finish = self
            .state
            .finish(
                &mut seeded_rng()?,
                &self.code,
                response,
                ClientLoginFinishParameters::new(Some(&self.context), identities(), Some(&ksf)),
            )
            .map_err(|_| ClientError::AuthenticationFailed)?;
        finish
            .message
            .serialize()
            .as_slice()
            .try_into()
            .map_err(|_| ClientError::Internal)
    }
}

/// Seeds a generator from the OS, reporting an entropy failure instead of
/// panicking like `UnwrapErr(SysRng)` would.
fn seeded_rng() -> Result<StdRng, ClientError> {
    StdRng::try_from_rng(&mut SysRng).map_err(|_| ClientError::Internal)
}
