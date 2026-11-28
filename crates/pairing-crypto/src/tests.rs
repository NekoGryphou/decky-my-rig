use super::session::{ApprovedSession, Challenge};
use super::*;
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientLoginStartResult, CredentialRequest,
};
use rand::{rand_core::UnwrapErr, rngs::SysRng};

const CODE: &[u8] = b"12345678";
const CERTIFICATE: &[u8] = b"fixture certificate DER";
const NONCE: [u8; 32] = [1; 32];

/// The confirmation a client computes for `challenge` with its own view of
/// the code, TLS certificate, session nonce and identities.
fn confirm(
    client: ClientLoginStartResult<Suite>,
    challenge: &Challenge,
    code: &[u8],
    context: &[u8],
    ids: Identifiers<'_>,
) -> Option<Vec<u8>> {
    let ksf = stretch();
    client
        .state
        .finish(
            &mut UnwrapErr(SysRng),
            code,
            decode_response(&challenge.response).ok()?,
            ClientLoginFinishParameters::new(Some(context), ids, Some(&ksf)),
        )
        .ok()
        .map(|finish| finish.message.serialize().to_vec())
}

/// Whether the host session authenticates a client with this view.
fn authenticates(code: &[u8], certificate: &[u8], nonce: &[u8; 32], ids: Identifiers<'_>) -> bool {
    let mut session = ApprovedSession::prepare(CODE, &NONCE, CERTIFICATE).unwrap();
    let client = ClientLogin::<Suite>::start(&mut UnwrapErr(SysRng), code).unwrap();
    let challenge = session.start(&client.message.serialize()).unwrap();
    let context = context(certificate, nonce);
    let Some(confirmation) = confirm(client, &challenge, code, &context, ids) else {
        return false;
    };
    session.finish(challenge.attempt, &confirmation).is_ok()
}

#[test]
fn only_the_matching_code_certificate_nonce_and_identity_authenticate() {
    eprintln!(
        "wire widths: request={REQUEST_LEN}, response={RESPONSE_LEN}, confirmation={CONFIRMATION_LEN}"
    );
    assert!(authenticates(CODE, CERTIFICATE, &NONCE, identities()));
    assert!(!authenticates(
        b"87654321",
        CERTIFICATE,
        &NONCE,
        identities()
    ));
    assert!(!authenticates(
        CODE,
        b"different certificate",
        &NONCE,
        identities()
    ));
    let mut other_nonce = NONCE;
    other_nonce[0] ^= 1;
    assert!(!authenticates(
        CODE,
        CERTIFICATE,
        &other_nonce,
        identities()
    ));
    let other_client = Identifiers {
        client: Some(b"different-client"),
        server: identities().server,
    };
    assert!(!authenticates(CODE, CERTIFICATE, &NONCE, other_client));
}

#[test]
fn malformed_wire_messages_rejected() {
    for bytes in [&[][..], &[0u8][..], &[0u8; 4096][..]] {
        assert!(CredentialRequest::<Suite>::deserialize(bytes).is_err());
        assert!(CredentialResponse::<Suite>::deserialize(bytes).is_err());
        assert!(decode_finalization(bytes).is_err());
        assert!(decode_response(bytes).is_err());
    }
    // ? The library's finalization parsing accepts trailing bytes, so test our strict check
    // ? directly with an otherwise valid MAC frame.
    assert!(decode_finalization(&[0; 64]).is_ok());
    assert!(decode_finalization(&[0; 65]).is_err());
    assert!(decode_finalization(&[0; 63]).is_err());
}

#[test]
fn finalization_from_previous_exchange_rejected() {
    let mut session = ApprovedSession::prepare(CODE, &NONCE, CERTIFICATE).unwrap();
    let context = context(CERTIFICATE, &NONCE);
    let first = ClientLogin::<Suite>::start(&mut UnwrapErr(SysRng), CODE).unwrap();
    let first_challenge = session.start(&first.message.serialize()).unwrap();
    let stale = confirm(first, &first_challenge, CODE, &context, identities()).unwrap();
    assert!(session.finish(first_challenge.attempt, &[0; 64]).is_err());
    let second = ClientLogin::<Suite>::start(&mut UnwrapErr(SysRng), CODE).unwrap();
    let second_challenge = session.start(&second.message.serialize()).unwrap();
    assert!(session.finish(second_challenge.attempt, &stale).is_err());
}
