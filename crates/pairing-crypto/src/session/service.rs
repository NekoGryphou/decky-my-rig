use super::{
    consts::{ATTEMPT_LIFETIME, LIFETIME, MAX_STARTS, RECORD_ID},
    errors::Error,
    types::{ApprovedSession, AttemptId, Authenticated, Challenge, Exchange},
};
use crate::{REQUEST_LEN, Suite, context, decode_finalization, identities, stretch};
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, CredentialRequest, ServerLogin,
    ServerLoginParameters, ServerRegistration, ServerSetup,
};
use rand::{Rng, rand_core::UnwrapErr, rngs::SysRng};
use std::time::Instant;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

impl AttemptId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
impl ApprovedSession {
    /// The local caller reserves its worker slot before calling this, checks
    /// approval, generates the code and nonce itself, and passes the real TLS
    /// certificate. No remote caller can register a session. If the OS random
    /// source panics on an entropy failure, don't fall back to weaker
    /// randomness or keep a half-built session.
    pub fn prepare(code: &[u8], nonce: &[u8; 32], certificate_der: &[u8]) -> Result<Self, Error> {
        if code.len() != 8 || !code.iter().all(u8::is_ascii_digit) || certificate_der.is_empty() {
            return Err(Error::InvalidInput);
        }
        let mut rng = UnwrapErr(SysRng);
        let setup = ServerSetup::<Suite>::new(&mut rng);
        let client =
            ClientRegistration::<Suite>::start(&mut rng, code).map_err(|_| Error::Crypto)?;
        let registration = ServerRegistration::start(&setup, client.message, RECORD_ID)
            .map_err(|_| Error::Crypto)?;
        let ksf = stretch();
        let mut finish = client
            .state
            .finish(
                &mut rng,
                code,
                registration.message,
                ClientRegistrationFinishParameters::new(identities(), Some(&ksf)),
            )
            .map_err(|_| Error::Crypto)?;
        finish.export_key.zeroize();
        Ok(Self {
            setup: Some(setup),
            record: Some(ServerRegistration::finish(finish.message)),
            context: context(certificate_der, nonce),
            until: Instant::now() + LIFETIME,
            starts: 0,
            exchange: None,
        })
    }
    pub fn close(&mut self) {
        self.exchange = None;
        self.record = None;
        self.setup = None;
    }
    /// A public monotonic deadline, not key material. The owner can use it to
    /// clean up idle sessions without starting an exchange.
    pub fn expires_at(&self) -> Instant {
        self.until
    }
    /// True until the first start, meaning nobody has guessed this code yet.
    pub fn untouched(&self) -> bool {
        self.starts == 0
    }
    /// True once every guess is spent. Such a code can't be guessed again, so
    /// the PC may replace it.
    pub fn exhausted(&self) -> bool {
        self.starts >= MAX_STARTS
    }
    fn check(&mut self, now: Instant) -> Result<(), Error> {
        if now >= self.until {
            self.close();
        }
        if self.setup.is_none() {
            return Err(Error::Closed);
        }
        if self.exchange.as_ref().is_some_and(|e| now >= e.until) {
            self.exchange = None;
        }
        Ok(())
    }
    pub fn start(&mut self, request: &[u8]) -> Result<Challenge, Error> {
        self.start_at(request, Instant::now())
    }
    pub(super) fn start_at(&mut self, request: &[u8], now: Instant) -> Result<Challenge, Error> {
        self.check(now)?;
        if self.exchange.is_some() {
            return Err(Error::Busy);
        }
        if self.starts >= MAX_STARTS {
            return Err(Error::Limit);
        }
        // ! Count the guess before parsing and before any reply, even if the client disconnects.
        self.starts += 1;
        if request.len() != REQUEST_LEN {
            return Err(Error::InvalidInput);
        }
        let request =
            CredentialRequest::<Suite>::deserialize(request).map_err(|_| Error::InvalidInput)?;
        let mut rng = UnwrapErr(SysRng);
        let mut id = [0; 32];
        rng.fill_bytes(&mut id);
        let login = ServerLogin::start(
            &mut rng,
            self.setup.as_ref().ok_or(Error::Closed)?,
            self.record.clone(),
            request,
            RECORD_ID,
            ServerLoginParameters {
                context: Some(&self.context),
                identifiers: identities(),
            },
        )
        .map_err(|_| Error::Crypto)?;
        let response = login
            .message
            .serialize()
            .as_slice()
            .try_into()
            .map_err(|_| Error::Crypto)?;
        let attempt = AttemptId(id);
        self.exchange = Some(Exchange {
            id: attempt,
            until: (now + ATTEMPT_LIFETIME).min(self.until),
            login: login.state,
        });
        Ok(Challenge { attempt, response })
    }
    /// Whether `id` is this session's open attempt. It doesn't cost a guess, so
    /// the owner can refuse an unknown attempt without claiming the session.
    pub fn holds_attempt(&self, id: AttemptId) -> bool {
        self.exchange
            .as_ref()
            .is_some_and(|exchange| bool::from(exchange.id.ct_eq(&id)))
    }
    pub fn finish(&mut self, id: AttemptId, confirmation: &[u8]) -> Result<Authenticated, Error> {
        self.finish_at(id, confirmation, Instant::now())
    }
    pub(super) fn finish_at(
        &mut self,
        id: AttemptId,
        confirmation: &[u8],
        now: Instant,
    ) -> Result<Authenticated, Error> {
        self.check(now)?;
        if self
            .exchange
            .as_ref()
            .is_none_or(|e| !bool::from(e.id.ct_eq(&id)))
        {
            return Err(Error::StaleAttempt);
        }
        // ! A correct attempt is used up even if the confirmation is malformed or fails.
        let exchange = self.exchange.take().ok_or(Error::StaleAttempt)?;
        let confirmation = decode_finalization(confirmation).map_err(|_| Error::InvalidInput)?;
        let mut result = exchange
            .login
            .finish(
                confirmation,
                ServerLoginParameters {
                    context: Some(&self.context),
                    identifiers: identities(),
                },
            )
            .map_err(|_| Error::Crypto)?;
        result.session_key.zeroize();
        self.close();
        Ok(Authenticated { _private: () })
    }
}
