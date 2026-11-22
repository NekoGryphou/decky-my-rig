use super::consts::ATTEMPT_LIFETIME;
use super::*;
use crate::decode_response;
use crate::{Suite, context, identities, stretch};
use opaque_ke::{ClientLogin, ClientLoginFinishParameters};
use rand::rngs::OsRng;
use std::time::{Duration, Instant};
fn session() -> ApprovedSession {
    ApprovedSession::prepare(b"12345678", &[1; 32], b"certificate").unwrap()
}
fn client(code: &[u8]) -> opaque_ke::ClientLoginStartResult<Suite> {
    ClientLogin::<Suite>::start(&mut OsRng, code).unwrap()
}
fn confirmation(
    c: opaque_ke::ClientLoginStartResult<Suite>,
    ch: &Challenge,
    cert: &[u8],
) -> Result<Vec<u8>, ()> {
    let ctx = context(cert, &[1; 32]);
    let ksf = stretch();
    c.state
        .finish(
            &mut OsRng,
            b"12345678",
            decode_response(&ch.response)?,
            ClientLoginFinishParameters::new(Some(&ctx), identities(), Some(&ksf)),
        )
        .map(|r| r.message.serialize().to_vec())
        .map_err(|_| ())
}
#[test]
fn success_one_shot_stale_and_replay() {
    let mut s = session();
    let c = client(b"12345678");
    let ch = s.start(&c.message.serialize()).unwrap();
    assert!(matches!(
        s.finish(AttemptId([0; 32]), &[0; 64]),
        Err(Error::StaleAttempt)
    ));
    let msg = confirmation(c, &ch, b"certificate").unwrap();
    assert!(s.finish(ch.attempt, &msg).is_ok());
    assert!(matches!(s.finish(ch.attempt, &msg), Err(Error::Closed)));
}
#[test]
fn wrong_code_context_and_retry() {
    let mut s = session();
    let c = client(b"87654321");
    let ch = s.start(&c.message.serialize()).unwrap();
    assert!(confirmation(c, &ch, b"certificate").is_err());
    assert!(s.finish(ch.attempt, &[0; 64]).is_err());
    let c = client(b"12345678");
    let ch = s.start(&c.message.serialize()).unwrap();
    assert!(confirmation(c, &ch, b"other").is_err());
    assert!(s.finish(ch.attempt, &[0; 64]).is_err());
    let c = client(b"12345678");
    let ch = s.start(&c.message.serialize()).unwrap();
    assert!(
        s.finish(ch.attempt, &confirmation(c, &ch, b"certificate").unwrap())
            .is_ok()
    );
}
#[test]
fn limits_framing_disconnects_and_closure() {
    let mut s = session();
    // - The malformed start counts too, since the guess is charged before
    //   parsing.
    assert!(matches!(s.start(&[0; 97]), Err(Error::InvalidInput)));
    for _ in 1..super::consts::MAX_STARTS {
        let c = client(b"12345678");
        let ch = s.start(&c.message.serialize()).unwrap();
        assert!(matches!(s.start(&c.message.serialize()), Err(Error::Busy)));
        assert!(matches!(
            s.finish(ch.attempt, &[0; 65]),
            Err(Error::InvalidInput)
        ));
    }
    assert!(matches!(s.start(&[0; 96]), Err(Error::Limit)));
    s.close();
    assert!(matches!(s.start(&[]), Err(Error::Closed)));
    assert!(matches!(
        ApprovedSession::prepare(b"123", &[0; 32], b"cert"),
        Err(Error::InvalidInput)
    ));
}
#[test]
fn expiry_boundaries_and_stale_id() {
    let mut s = session();
    let now = Instant::now();
    let c = client(b"12345678");
    let old = s.start_at(&c.message.serialize(), now).unwrap();
    assert!(matches!(
        s.finish_at(old.attempt, &[0; 64], now + ATTEMPT_LIFETIME),
        Err(Error::StaleAttempt)
    ));
    let new = s
        .start_at(&c.message.serialize(), now + ATTEMPT_LIFETIME)
        .unwrap();
    assert!(matches!(
        s.finish(old.attempt, &[0; 64]),
        Err(Error::StaleAttempt)
    ));
    assert!(s.exchange.as_ref().is_some_and(|e| e.id == new.attempt));
    s.until = now + Duration::from_secs(16);
    assert!(matches!(
        s.finish_at(new.attempt, &[0; 64], s.until),
        Err(Error::Closed)
    ));
}
#[test]
fn replay_rejection_and_clamped_last_attempt() {
    let mut s = session();
    let c = client(b"12345678");
    let first = s.start(&c.message.serialize()).unwrap();
    let replay = confirmation(c, &first, b"certificate").unwrap();
    assert!(s.finish(first.attempt, &[0; 64]).is_err());
    let c = client(b"12345678");
    let second = s.start(&c.message.serialize()).unwrap();
    assert!(first.attempt != second.attempt);
    assert!(matches!(
        s.finish(first.attempt, &replay),
        Err(Error::StaleAttempt)
    ));
    assert!(matches!(
        s.finish(second.attempt, &replay),
        Err(Error::Crypto)
    ));
    assert!(s.exchange.is_none());
    let now = Instant::now();
    s.until = now + Duration::from_secs(1);
    let last = s.start_at(&c.message.serialize(), now).unwrap();
    assert!(s.exchange.as_ref().is_some_and(|e| e.until == s.until));
    assert!(matches!(
        s.finish_at(last.attempt, &[0; 64], s.until),
        Err(Error::Closed)
    ));
}
