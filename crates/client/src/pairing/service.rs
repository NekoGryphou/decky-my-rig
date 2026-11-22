use super::{
    consts::{
        BOOTSTRAP_VERSION, BUSY_RETRY, BUSY_STATUS, CODE_REQUEST_DEADLINE, CODE_SHOWN_STATUSES,
        PAIR_DEADLINE, REVOKE_DEADLINE,
    },
    messages::{CODE_EXPIRED, CODE_INCORRECT, CODE_NOT_SHOWN, FAILED, UNREACHABLE},
    types::{Failure, Paired},
    utils::{bootstrap_reply, public_message, remaining},
};
use crate::{
    host_client::{HostClient, HostError, HttpReply},
    shared::{PublicError, normalize_name, normalize_token},
    store::Authentication,
};
use hyper::Method;
use myrig_pairing_crypto::{RESPONSE_LEN, client::ClientExchange};
use myrig_protocol::{
    lan::{
        PAIR_FINISH_PATH, PAIR_INFO_PATH, PAIR_REQUEST_PATH, PAIR_START_PATH, PairCodeRequest,
        PairFinishReply, PairFinishRequest, PairInfoReply, PairStartReply, PairStartRequest,
    },
    validation::{canonical_uuid, decode_hex, lower_hex, valid_pairing_code},
};
use sha2::{Digest, Sha256};
use std::{net::Ipv4Addr, time::Instant};

/// Asks the PC to show a pairing code to whoever is sitting in front of it.
/// This is sent before the PC is trusted, so it only carries this device's
/// name; the code itself never goes over the network.
pub async fn request_code(host: Ipv4Addr, port: u16, device_name: &str) -> Result<(), PublicError> {
    if normalize_name(device_name).ok().as_deref() != Some(device_name) {
        return Err(PublicError(CODE_NOT_SHOWN));
    }
    let body = serde_json::to_vec(&PairCodeRequest {
        device_name: device_name.to_owned(),
    })
    .map_err(|_| PublicError(CODE_NOT_SHOWN))?;
    let client = HostClient::new(host, port, None, None, CODE_REQUEST_DEADLINE);
    match client
        .request_unverified(Method::POST, PAIR_REQUEST_PATH, Some(body))
        .await
    {
        Ok((reply, _)) if CODE_SHOWN_STATUSES.contains(&reply.status) => Ok(()),
        Err(HostError::Connection | HostError::Timeout) => Err(PublicError(UNREACHABLE)),
        _ => Err(PublicError(CODE_NOT_SHOWN)),
    }
}

/// Best-effort revoke of a credential the PC issued that this device won't
/// save, so the PC doesn't keep a device that can never use it.
pub async fn revoke(host: Ipv4Addr, port: u16, authentication: Authentication) {
    let client = HostClient::new(
        host,
        port,
        Some(authentication.token.to_string()),
        Some(authentication.tls_pin),
        REVOKE_DEADLINE,
    );
    let _ = client.unpair().await;
}

/// Does one exchange with the code shown on the PC, without retrying.
/// `previous` is the (pin, token) of a credential this pairing replaces; the
/// token only goes to that same certificate, after the PC has proved it knows
/// the code. The caller has to save the result, since the PC has already saved
/// its side.
pub async fn pair(
    host: Ipv4Addr,
    port: u16,
    code: &str,
    previous: Option<(&[u8; 32], &str)>,
) -> Result<Paired, PublicError> {
    if !valid_pairing_code(code) {
        return Err(PublicError(FAILED));
    }
    let mut finishing = false;
    exchange(
        host,
        port,
        code,
        previous,
        Instant::now() + PAIR_DEADLINE,
        &mut finishing,
    )
    .await
    .map_err(|failure| PublicError(public_message(failure, finishing)))
}

async fn exchange(
    host: Ipv4Addr,
    port: u16,
    code: &str,
    previous: Option<(&[u8; 32], &str)>,
    deadline: Instant,
    finishing: &mut bool,
) -> Result<Paired, Failure> {
    let (reply, certificate) = loop {
        let unverified = HostClient::new(host, port, None, None, remaining(deadline)?);
        let answer = unverified
            .request_unverified(Method::GET, PAIR_INFO_PATH, None)
            .await
            .map_err(Failure::Host)?;
        if !wait_if_busy(&answer.0, deadline).await {
            break answer;
        }
    };
    let info: PairInfoReply = step_reply(&reply, &["bootstrap_version", "nonce"], false)?;
    let nonce = decode_hex::<32>(&info.nonce).ok_or(Failure::Public(FAILED))?;
    if info.bootstrap_version != BOOTSTRAP_VERSION {
        return Err(Failure::Public(FAILED));
    }
    // ! Not trusted until the exchange authenticates it. The next two requests
    //   have to see this same pin.
    let pin: [u8; 32] = Sha256::digest(&certificate).into();
    let (pending, request) = ClientExchange::start(code.as_bytes(), &certificate, &nonce)
        .map_err(|_| Failure::Public(FAILED))?;
    let pinned = |deadline| -> Result<HostClient, Failure> {
        Ok(HostClient::new(
            host,
            port,
            None,
            Some(pin),
            remaining(deadline)?,
        ))
    };
    let start = json_body(&PairStartRequest {
        request: lower_hex(&request),
    })?;
    // ! Retrying is safe here: a busy start never reached the code, so it didn't
    //   use up a guess. Finish is never repeated.
    let reply = loop {
        let reply = pinned(deadline)?
            .request(Method::POST, PAIR_START_PATH, Some(start.clone()))
            .await
            .map_err(Failure::Host)?;
        if !wait_if_busy(&reply, deadline).await {
            break reply;
        }
    };
    let started: PairStartReply =
        step_reply(&reply, &["attempt_id", "session_nonce", "response"], false)?;
    let attempt = decode_hex::<32>(&started.attempt_id).ok_or(Failure::Public(FAILED))?;
    let response = decode_hex::<RESPONSE_LEN>(&started.response).ok_or(Failure::Public(FAILED))?;
    if decode_hex::<32>(&started.session_nonce) != Some(nonce) {
        return Err(Failure::Public(FAILED));
    }
    remaining(deadline)?;
    // ? The memory-hard finish runs off the async thread. It's short and bounded,
    //   and a late result is thrown away below before anything is sent.
    let confirmation = tokio::task::spawn_blocking(move || pending.finish(&response))
        .await
        .map_err(|_| Failure::Public(FAILED))?
        .map_err(|_| Failure::Public(CODE_INCORRECT))?;
    let finish = json_body(&PairFinishRequest {
        attempt_id: lower_hex(&attempt),
        confirmation: lower_hex(&confirmation),
        previous_token: previous
            .filter(|(previous_pin, _)| **previous_pin == pin)
            .map(|(_, token)| token.to_owned()),
    })?;
    let client = pinned(deadline)?;
    *finishing = true;
    let reply = client
        .request(Method::POST, PAIR_FINISH_PATH, Some(finish))
        .await
        .map_err(Failure::Host)?;
    let finished: PairFinishReply =
        step_reply(&reply, &["host_id", "token", "computer_name"], true)?;
    // * Once finish answers, the PC has saved this device, so a late answer is
    //   still kept.
    let host_id = canonical_uuid(&finished.host_id).ok_or(Failure::Public(FAILED))?;
    let token = normalize_token(&finished.token)
        .ok()
        .filter(|token| *token == finished.token)
        .ok_or(Failure::Public(FAILED))?;
    let computer_name = normalize_name(&finished.computer_name)
        .ok()
        .filter(|name| *name == finished.computer_name)
        .ok_or(Failure::Public(FAILED))?;
    Ok(Paired {
        authentication: Authentication {
            host_id,
            token: zeroize::Zeroizing::new(token),
            tls_pin: pin,
        },
        computer_name,
    })
}

/// If the PC said it's busy and there's time left, waits before asking again.
async fn wait_if_busy(reply: &HttpReply, deadline: Instant) -> bool {
    if reply.status != BUSY_STATUS || Instant::now() + BUSY_RETRY >= deadline {
        return false;
    }
    tokio::time::sleep(BUSY_RETRY).await;
    true
}

/// A closed or expired code gets a 403 before finish. After finish, a 403 means
/// the exchange itself failed.
fn step_reply<T: serde::de::DeserializeOwned>(
    reply: &HttpReply,
    fields: &[&str],
    finish: bool,
) -> Result<T, Failure> {
    if reply.status == 403 && !finish {
        return Err(Failure::Public(CODE_EXPIRED));
    }
    bootstrap_reply(reply, fields)
}

fn json_body<T: serde::Serialize>(body: &T) -> Result<Vec<u8>, Failure> {
    serde_json::to_vec(body).map_err(|_| Failure::Public(FAILED))
}
