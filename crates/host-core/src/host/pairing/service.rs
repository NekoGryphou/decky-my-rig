use super::super::{errors::*, types::*, utils::session_pending};
use myrig_pairing_crypto::{
    CONFIRMATION_LEN, REQUEST_LEN,
    session::{ApprovedSession, AttemptId, Error as CryptoError},
};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, MutexGuard, PoisonError},
    time::Instant,
};

impl Drop for PairingReservation {
    fn drop(&mut self) {
        // ! Releasing a claim mustn't depend on other code not panicking. A
        //   poisoned lock still holds valid flags, and stopping the service waits
        //   on this.
        self.0
            .pairing
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .busy = false;
        self.0.pairing_finished.notify_waiters();
    }
}

impl Host {
    fn pairing_state(&self) -> Option<MutexGuard<'_, PairingState>> {
        self.pairing.lock().ok()
    }

    pub fn pairing_approval(&self) -> PairingApproval {
        PairingApproval(
            self.pairing_state()
                .map_or(u64::MAX, |state| state.generation),
        )
    }

    /// Only for trusted callers. The caller provides local approval, the real
    /// TLS certificate, and a fresh OS-random code, nonce and credential. No
    /// route exposes this, and the pairing lock is never held during crypto or
    /// saves.
    // ? These are the inputs listed above. There's only one production caller, so
    //   a wrapper struct wouldn't help.
    #[allow(clippy::too_many_arguments)]
    pub async fn open_secure_pairing(
        self: &Arc<Self>,
        approval: PairingApproval,
        name: &str,
        code: &str,
        nonce: [u8; 32],
        certificate_der: &[u8],
        entropy: [u8; 32],
        device_id: uuid::Uuid,
        persist: PairingPersistence,
    ) -> Result<(), OpenPairingError> {
        if !myrig_protocol::validation::valid_name(name)
            || !myrig_protocol::validation::valid_pairing_code(code)
            || certificate_der.is_empty()
        {
            return Err(OpenPairingError::InvalidInput);
        }
        if self
            .pairing_state()
            .ok_or(OpenPairingError::Unavailable)?
            .busy
        {
            return Err(OpenPairingError::Busy);
        }
        let registry = {
            // ? A claimed save can hold registry_mutation while it waits for
            //   storage. Don't make the async caller wait behind it just to find
            //   out it's busy.
            let _mutation = self
                .registry_mutation
                .try_lock()
                .map_err(|error| match error {
                    std::sync::TryLockError::WouldBlock => OpenPairingError::Busy,
                    std::sync::TryLockError::Poisoned(_) => OpenPairingError::Unavailable,
                })?;
            if !self.storage_healthy() {
                return Err(OpenPairingError::RecoveryRequired);
            }
            let digest = Sha256::digest(entropy).into();
            let mut candidate = self
                .config
                .read()
                .map_err(|_| OpenPairingError::Unavailable)?
                .clone();
            candidate
                .insert_credential(device_id, name, digest)
                .map_err(|_| OpenPairingError::RegistryRejected)?;
            let mut state = self.pairing_state().ok_or(OpenPairingError::Unavailable)?;
            if state.stopped || approval.0 != state.generation {
                return Err(OpenPairingError::Unavailable);
            }
            if state.busy {
                return Err(OpenPairingError::Busy);
            }
            // ! A code someone has guessed at stays until it expires or is
            //   closed, so nobody can swap it for a fresh one to get more
            //   guesses.
            if session_pending(&mut state, Instant::now())
                && state
                    .session
                    .as_ref()
                    .is_some_and(|session| !session.crypto.untouched())
            {
                return Err(OpenPairingError::Busy);
            }
            // * Replacing an unclaimed approval drops its code and credential
            //   before the next one is prepared. A preparation or save in
            //   progress keeps ownership.
            state.session = None;
            state.closed = false;
            state.busy = true;
            RegistryPairing {
                device_id,
                name: name.to_owned(),
                digest,
                persist,
            }
        };
        let reservation = PairingReservation(self.clone());
        let code = zeroize::Zeroizing::new(code.as_bytes().to_vec());
        let entropy = zeroize::Zeroizing::new(entropy);
        let cert = certificate_der.to_vec();
        let leaf_digest = Sha256::digest(certificate_der).into();
        tokio::task::spawn_blocking(move || {
            let crypto = ApprovedSession::prepare(&code, &nonce, &cert)
                .map_err(|_| OpenPairingError::Unavailable)?;
            let token = zeroize::Zeroizing::new(myrig_protocol::validation::lower_hex(&*entropy));
            reservation.0.restore_secure(SecurePairing {
                crypto,
                leaf_digest,
                nonce,
                token,
                registry,
            })
        })
        .await
        .map_err(|_| OpenPairingError::Unavailable)?
    }

    /// Publishes a session the caller still holds a reservation for.
    pub(crate) fn restore_secure(&self, session: SecurePairing) -> Result<(), OpenPairingError> {
        let mut state = self.pairing_state().ok_or(OpenPairingError::Unavailable)?;
        if state.stopped || state.closed {
            return Err(OpenPairingError::Unavailable);
        }
        state.session = Some(session);
        Ok(())
    }

    /// Call this periodically to drop idle, expired secrets.
    pub fn expire_pending_pairing(&self) {
        self.expire_pending_pairing_at(Instant::now());
    }
    pub(crate) fn expire_pending_pairing_at(&self, now: Instant) {
        if let Some(mut state) = self.pairing_state() {
            // * A claimed session belongs to its worker, not to this slot.
            if !state.busy {
                session_pending(&mut state, now);
            }
        }
    }

    /// Takes the open session for one step. If `attempt` is given and the
    /// session doesn't hold it, the call is refused without claiming the
    /// session, so an unknown attempt can't keep the code busy.
    pub(crate) fn claim_secure(
        self: &Arc<Self>,
        binding: [u8; 32],
        attempt: Option<AttemptId>,
    ) -> Result<(SecurePairing, PairingReservation), PairingRefusal> {
        let mut state = self.pairing_state().ok_or(PairingRefusal::Unavailable)?;
        if state.busy {
            return Err(PairingRefusal::Busy);
        }
        if !self.storage_healthy() {
            return Err(PairingRefusal::Unavailable);
        }
        // ! The code was opened for this host's certificate, so a code from
        //   before an identity change isn't used. The OPAQUE context ties the
        //   exchange to that certificate and the nonce.
        let usable = state.session.as_ref().is_some_and(|s| {
            s.leaf_digest == binding && attempt.is_none_or(|id| s.crypto.holds_attempt(id))
        });
        if !usable {
            return Err(PairingRefusal::Unavailable);
        }
        let session = state.session.take().ok_or(PairingRefusal::Unavailable)?;
        state.busy = true;
        Ok((session, PairingReservation(self.clone())))
    }

    pub(crate) async fn secure_pairing_start(
        self: &Arc<Self>,
        request: &[u8],
        binding: [u8; 32],
    ) -> Result<SecureChallenge, PairingRefusal> {
        // ! Check the size before taking ownership. A malformed request still
        //   uses up an attempt, by sending a fixed, invalid empty frame.
        let request = if request.len() == REQUEST_LEN {
            request.to_vec()
        } else {
            Vec::new()
        };
        let (mut session, reservation) = self.claim_secure(binding, None)?;
        // ! Every guess counts until a pairing succeeds, whatever code it was
        //   for.
        if let Some(mut state) = self.pairing_state() {
            state.failed_starts = state.failed_starts.saturating_add(1);
        }
        tokio::task::spawn_blocking(move || {
            let challenge = match session.crypto.start(&request) {
                Err(CryptoError::Closed) => return None,
                // ! A used-up code is kept, closed to guesses, until it expires.
                //   Dropping it would let a new request open a fresh one.
                Err(CryptoError::Limit) => {
                    let _ = reservation.0.restore_secure(session);
                    return None;
                }
                result => result.ok(),
            };
            let nonce = session.nonce;
            let restored = reservation.0.restore_secure(session).is_ok();
            if restored {
                challenge.map(|challenge| SecureChallenge { nonce, challenge })
            } else {
                None
            }
        })
        .await
        .ok()
        .flatten()
        .ok_or(PairingRefusal::Unavailable)
    }

    pub(crate) async fn secure_pairing_finish(
        self: &Arc<Self>,
        attempt: AttemptId,
        confirmation: &[u8],
        binding: [u8; 32],
        previous: Option<zeroize::Zeroizing<String>>,
    ) -> Result<zeroize::Zeroizing<String>, PairingRefusal> {
        let confirmation = if confirmation.len() == CONFIRMATION_LEN {
            confirmation.to_vec()
        } else {
            Vec::new()
        };
        let (mut session, reservation) = self.claim_secure(binding, Some(attempt))?;
        tokio::task::spawn_blocking(move || {
            match session.crypto.finish(attempt, &confirmation) {
                Err(CryptoError::Closed) => return None,
                Err(_) => {
                    let _ = reservation.0.restore_secure(session);
                    return None;
                }
                Ok(_authenticated) => {}
            }
            // ! Authentication consumes the OPAQUE session before issuing
            //   anything. Closing before this check drops unclaimed work; a save
            //   that was already let in is waited for.
            if reservation.0.pairing_state()?.closed {
                return None;
            }
            reservation
                .0
                .issue_registry_token(session.token, session.registry, previous)
        })
        .await
        .ok()
        .flatten()
        .ok_or(PairingRefusal::Unavailable)
    }

    pub fn secure_pairing_info(&self, binding: [u8; 32]) -> Result<[u8; 32], PairingRefusal> {
        let mut state = self.pairing_state().ok_or(PairingRefusal::Unavailable)?;
        if state.busy {
            return Err(PairingRefusal::Busy);
        }
        if !self.storage_healthy() || state.closed || !session_pending(&mut state, Instant::now()) {
            return Err(PairingRefusal::Unavailable);
        }
        state
            .session
            .as_ref()
            .filter(|s| s.leaf_digest == binding)
            .map(|s| s.nonce)
            .ok_or(PairingRefusal::Unavailable)
    }

    pub(crate) fn issue_registry_token(
        &self,
        token: zeroize::Zeroizing<String>,
        registry: RegistryPairing,
        previous: Option<zeroize::Zeroizing<String>>,
    ) -> Option<zeroize::Zeroizing<String>> {
        let _mutation = self.registry_mutation.lock().ok()?;
        if !self.storage_healthy() {
            return None;
        }
        let mut candidate = self.config.read().ok()?.clone();
        // * A re-pairing device names its old credential, which gets revoked in
        //   the same save that adds the new one.
        if let Some(previous) = previous {
            candidate.revoke_token(&previous);
        }
        candidate
            .insert_credential(registry.device_id, &registry.name, registry.digest)
            .ok()?;
        let bytes = candidate.to_json().ok()?;
        let saved =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (registry.persist)(&bytes)));
        let saved = match saved {
            Ok(saved) => saved,
            Err(_) => {
                self.require_storage_recovery();
                return None;
            }
        };
        if let Err(error) = saved {
            if error != PersistError::Unchanged {
                self.require_storage_recovery();
            }
            return None;
        }
        match self.config.write() {
            Ok(mut config) => *config = candidate,
            Err(_) => {
                self.require_storage_recovery();
                return None;
            }
        }
        if let Some(mut state) = self.pairing_state() {
            state.failed_starts = 0;
        }
        // ! The caller only gets the bearer token once the save succeeded.
        Some(token)
    }

    /// Stopping the service waits for a save that has already started, even if
    /// its HTTP client is gone. It doesn't cancel that save or put a deadline
    /// on it.
    pub async fn wait_pairing_idle(&self) {
        loop {
            let finished = self.pairing_finished.notified();
            tokio::pin!(finished);
            finished.as_mut().enable();
            if !self
                .pairing
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .busy
            {
                return;
            }
            finished.await;
        }
    }

    /// Only an unclaimed code can be closed; a claimed save is never canceled.
    /// Stop new openers before closing and waiting.
    pub fn close_pairing(&self) {
        if let Some(mut state) = self.pairing_state() {
            // ! This invalidates approval callbacks sent before the close, even
            //   if their worker hasn't reached the host yet.
            state.generation = state.generation.wrapping_add(1);
            state.closed = true;
            state.session = None;
        }
    }

    /// A permanent stop, unlike closing a code, which can be opened again
    /// later. An opener that's already preparing can't restore a session after
    /// this.
    pub fn stop_pairing(&self) {
        if let Some(mut state) = self.pairing_state() {
            state.stopped = true;
            state.generation = state.generation.wrapping_add(1);
            state.closed = true;
            state.session = None;
        }
    }
}
