use super::{ClientError, ClientExchange};
use crate::session::ApprovedSession;

const CODE: &[u8] = b"12345678";
const CERTIFICATE: &[u8] = b"fixture certificate DER";
const NONCE: [u8; 32] = [1; 32];

/// Runs one exchange against an approved host session. Returns whether the
/// client accepted the host and whether the host accepted the client.
fn exchange(code: &[u8], certificate: &[u8]) -> (Result<(), ClientError>, bool) {
    let mut session = ApprovedSession::prepare(CODE, &NONCE, CERTIFICATE).unwrap();
    let (client, request) = ClientExchange::start(code, certificate, &NONCE).unwrap();
    let challenge = session.start(&request).unwrap();
    match client.finish(&challenge.response) {
        Ok(confirmation) => (
            Ok(()),
            session.finish(challenge.attempt, &confirmation).is_ok(),
        ),
        Err(error) => (Err(error), false),
    }
}

#[test]
fn matching_code_and_certificate_authenticate_both_sides() {
    assert_eq!(exchange(CODE, CERTIFICATE), (Ok(()), true));
}

#[test]
fn wrong_code_or_certificate_fails_on_the_client() {
    assert_eq!(
        exchange(b"87654321", CERTIFICATE),
        (Err(ClientError::AuthenticationFailed), false)
    );
    assert_eq!(
        exchange(CODE, b"another certificate"),
        (Err(ClientError::AuthenticationFailed), false)
    );
}

#[test]
fn malformed_inputs_are_refused() {
    for (code, certificate) in [
        (&b"1234567"[..], CERTIFICATE),
        (b"123456789", CERTIFICATE),
        (b"1234567a", CERTIFICATE),
        (CODE, &b""[..]),
        (CODE, &vec![0u8; 65_537][..]),
    ] {
        assert_eq!(
            ClientExchange::start(code, certificate, &NONCE).err(),
            Some(ClientError::InvalidInput)
        );
    }
    let (client, _) = ClientExchange::start(CODE, CERTIFICATE, &NONCE).unwrap();
    assert_eq!(
        client.finish(&[0; 10]).err(),
        Some(ClientError::InvalidInput)
    );
}
