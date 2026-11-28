use super::super::{
    types::Reply,
    utils::{error, invalid, parse_checked, reply_body},
};
use crate::host::Host;
use crate::shutdown::ShutdownError;
use hyper::StatusCode;
use myrig_protocol::lan::{HostState, ShutdownReply, ShutdownRequest};
use myrig_protocol::validation::canonical_uuid;
use std::{sync::Arc, time::Instant};

pub(in crate::http) async fn handle(
    host: &Arc<Host>,
    bytes: &[u8],
    authenticated_device: String,
    require_test_mode: bool,
) -> Reply {
    let Some(id) = parse_checked::<ShutdownRequest>(bytes)
        .and_then(|request| canonical_uuid(&request.request_id))
    else {
        return invalid();
    };
    let accepted = |test| {
        reply_body(
            StatusCode::ACCEPTED,
            ShutdownReply {
                request_id: id.to_string(),
                state: HostState::ShuttingDown,
                test,
            },
        )
    };
    match host.test_mode() {
        Ok(true) => {
            host.events.record(
                myrig_protocol::events::EventKind::TestShutdown,
                authenticated_device,
            );
            return accepted(Some(true));
        }
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Host internal error.",
            );
        }
        Ok(false) if require_test_mode => {
            return error(
                StatusCode::CONFLICT,
                "test_mode_disabled",
                "Test mode is disabled.",
            );
        }
        Ok(false) => {}
    }
    // ! The OS shutdown call can stall, so it runs off the service's runtime thread, where /status
    // ! and stop would otherwise wait behind it.
    let shared = Arc::clone(host);
    let result =
        match tokio::task::spawn_blocking(move || shared.core.shutdown(id, Instant::now())).await {
            Ok(result) => result,
            Err(_) => Err(ShutdownError::Internal),
        };
    match result {
        Ok(()) => accepted(None),
        Err(ShutdownError::InProgress) => error(
            StatusCode::CONFLICT,
            "shutdown_in_progress",
            "Shutdown is already pending.",
        ),
        Err(ShutdownError::Failed(_)) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "shutdown_failed",
            "Windows did not accept shutdown.",
        ),
        Err(ShutdownError::Internal) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Host internal error.",
        ),
    }
}
