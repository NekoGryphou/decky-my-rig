use super::errors::ServiceLoopError;
use crate::host::Host;
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::watch;

pub async fn join_endpoints(
    host: Arc<Host>,
    http: impl Future<Output = Result<(), ()>>,
    control: impl Future<Output = Result<(), ()>>,
    stop: impl Future<Output = ()>,
    endpoint_stop: watch::Sender<bool>,
    on_stopping: impl FnOnce() -> Result<(), ()>,
) -> Result<(), ServiceLoopError> {
    tokio::pin!(http, control, stop);
    let mut expiry = tokio::time::interval(Duration::from_secs(1));
    let mut http_result = None;
    let mut control_result = None;
    let requested = loop {
        tokio::select! {
            biased;
            _ = &mut stop => break true,
            result = &mut http => { http_result = Some(result); break false; },
            result = &mut control => { control_result = Some(result); break false; },
            _ = expiry.tick() => host.expire_pending_pairing(),
        }
    };
    // ! Refuse new approvals before telling both endpoints. Crypto or storage work that was already
    // ! claimed is still owned and has to finish, even if the client left.
    host.stop_pairing();
    host.events().close();
    let status = on_stopping();
    let _ = endpoint_stop.send(true);
    let (http_result, control_result) = tokio::join!(
        async {
            match http_result {
                Some(result) => result,
                None => http.await,
            }
        },
        async {
            match control_result {
                Some(result) => result,
                None => control.await,
            }
        },
    );
    host.wait_pairing_idle().await;
    if status.is_err() {
        Err(ServiceLoopError::Status)
    } else if http_result.is_err() {
        Err(ServiceLoopError::Http)
    } else if control_result.is_err() {
        Err(ServiceLoopError::Control)
    } else if !requested {
        Err(ServiceLoopError::UnexpectedExit)
    } else {
        Ok(())
    }
}
