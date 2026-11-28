use super::{
    consts::{CAPACITY_UNAVAILABLE, DEADLINE, INVALID_RESPONSE, UNAUTHORIZED},
    native::duplicate_pipe,
    types::{Allowed, Authorize, Capacity, Dispatch, Worker},
};
use crate::shared::consts::FAILURE;
use myrig_protocol::local_control::framing::{read_frame, write_frame};
use myrig_protocol::local_control::{REQUEST_LIMIT, RESPONSE_LIMIT};
use std::{io, sync::Arc, thread};
use tokio::{
    io::AsyncReadExt,
    net::windows::named_pipe::NamedPipeServer,
    sync::{oneshot, watch},
    time::timeout,
};
use windows::Win32::Foundation::{ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED};

pub(super) fn peer_disconnected(error: &io::Error) -> bool {
    error.raw_os_error().is_some_and(|code| {
        [ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED]
            .iter()
            .any(|known| known.0 as i32 == code)
    })
}

fn disconnect(pipe: &NamedPipeServer) -> Result<(), &'static str> {
    match pipe.disconnect() {
        Ok(()) => Ok(()),
        Err(error) if peer_disconnected(&error) => Ok(()),
        Err(_) => Err(FAILURE),
    }
}

/// Answers a request this connection can't serve. It costs the connection,
/// never the pipe.
async fn refuse(pipe: &mut NamedPipeServer) -> Result<(), &'static str> {
    let _ = timeout(
        DEADLINE,
        write_frame(pipe, INVALID_RESPONSE, RESPONSE_LIMIT),
    )
    .await;
    Ok(())
}

pub(super) async fn serve_connection<Context: Send + 'static>(
    mut pipe: NamedPipeServer,
    dispatch: Dispatch<Context>,
    allowed: Arc<Allowed>,
    mut stop: watch::Receiver<bool>,
    capacity: Arc<Capacity>,
    reader: tokio::sync::OwnedSemaphorePermit,
    authorize: Authorize,
) -> Result<(), &'static str> {
    let request = tokio::select! {
        request = timeout(DEADLINE, read_frame(&mut pipe, REQUEST_LIMIT)) => request.ok().and_then(Result::ok),
        _ = stop.changed() => { disconnect(&pipe)?; return Ok(()); }
    };
    drop(reader);
    if let Some(request) = request {
        let slots = if myrig_host_core::local_control::is_event_wait_request(&request) {
            &capacity.events
        } else {
            &capacity.normal
        };
        let permit = Arc::clone(slots).try_acquire_owned().ok();
        // ! A resource failure costs this connection, never the pipe. Only a failed impersonation
        // ! revert (from `authorize`) is fatal.
        let Ok(duplicate) = duplicate_pipe(&pipe) else {
            return refuse(&mut pipe).await;
        };
        let (sender, receiver) = oneshot::channel();
        let thread = thread::Builder::new()
            .name("myrig-local-control".into())
            .spawn(move || {
                let result = match authorize(&duplicate, &allowed) {
                    Ok(true) => {
                        let Some(_permit) = permit else {
                            let _ = sender.send(Ok(CAPACITY_UNAVAILABLE.to_vec().into()));
                            return;
                        };
                        let response = (dispatch.callback)(&request, dispatch.context);
                        Ok(if response.is_empty() || response.len() > RESPONSE_LIMIT {
                            INVALID_RESPONSE.to_vec().into()
                        } else {
                            response
                        })
                    }
                    Ok(false) => Ok(UNAUTHORIZED.to_vec().into()),
                    Err(error) => Err(error),
                };
                let _ = sender.send(result);
            });
        let Ok(thread) = thread else {
            return refuse(&mut pipe).await;
        };
        let response = Worker {
            thread: Some(thread),
            response: receiver,
        }
        .finish()
        .await?;
        if *stop.borrow() {
            disconnect(&pipe)?;
            return Ok(());
        }
        let _ = timeout(DEADLINE, write_frame(&mut pipe, &response, RESPONSE_LIMIT)).await;
        // ! Let the client read and close. A second request is never run.
        let mut extra = [0u8; 1];
        tokio::select! {
            _ = timeout(DEADLINE, pipe.read(&mut extra)) => {},
            _ = stop.changed() => {},
        }
    }
    Ok(())
}
