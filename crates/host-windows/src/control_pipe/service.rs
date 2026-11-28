use super::consts::{EVENT_SLOTS, MAX_ACTIVE, NORMAL_SLOTS, READING_SLOTS};
use super::{
    native::create_pipe,
    types::{Capacity, ControlCallback, ControlPipe, Dispatch, Worker},
    utils::{peer_disconnected, serve_connection},
};
use crate::shared::consts::FAILURE;
use std::sync::Arc;
use tokio::sync::{Semaphore, watch};
use zeroize::Zeroizing;

impl Worker {
    pub(super) async fn finish(mut self) -> Result<Zeroizing<Vec<u8>>, &'static str> {
        let result = (&mut self.response).await.map_err(|_| FAILURE)?;
        // ? The thread has sent its reply and is ending. Joining it off the runtime thread keeps
        // ? the pipe's other connections running.
        let thread = self.thread.take().ok_or(FAILURE)?;
        tokio::task::spawn_blocking(move || thread.join())
            .await
            .map_err(|_| FAILURE)?
            .map_err(|_| FAILURE)?;
        result
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        // ! Canceling can't detach a change in progress or leave an impersonating OS thread
        // ! running. The callback has to finish its own transaction.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl ControlPipe {
    /// Each connected pipe handles one request with one worker, within fixed
    /// limits. On stop, running callbacks finish before the service reports
    /// it's done. Capture may only copy trusted cancellation data, and the
    /// caller is always authorized before the callback runs.
    pub async fn serve<F, Reply, Capture, Context>(
        mut self,
        callback: F,
        capture: Capture,
        mut stop: watch::Receiver<bool>,
    ) -> Result<(), &'static str>
    where
        F: Fn(&[u8], Context) -> Reply + Send + Sync + 'static,
        Reply: Into<Zeroizing<Vec<u8>>>,
        Capture: Fn() -> Context + Send + Sync + 'static,
        Context: Send + 'static,
    {
        // ! Replies can carry a pairing code, so they are wiped once written.
        let callback: ControlCallback<Context> =
            Arc::new(move |request, context| callback(request, context).into());
        let capacity = Arc::new(Capacity {
            normal: Arc::new(Semaphore::new(NORMAL_SLOTS)),
            events: Arc::new(Semaphore::new(EVENT_SLOTS)),
            reading: Arc::new(Semaphore::new(READING_SLOTS)),
        });
        let mut active = tokio::task::JoinSet::<Result<(), &'static str>>::new();
        let mut failure = None;
        loop {
            if *stop.borrow() {
                break;
            }
            tokio::select! {
                connected = async {
                    let reader = Arc::clone(&capacity.reading).acquire_owned().await
                        .map_err(|_| std::io::Error::other(FAILURE))?;
                    self.pipe.connect().await?;
                    Ok::<_, std::io::Error>(reader)
                }, if active.len() < MAX_ACTIVE => match connected {
                    Ok(reader) => {
                        // ! Take the snapshot before publishing the next instance, so a newer
                        // ! worker can't get ahead of this connection's cancellation state.
                        let dispatch = Dispatch { callback: Arc::clone(&callback), context: capture() };
                        // ! Returning early here would drop the connection tasks while the callback
                        // ! still needs this thread, so a failed instance ends the loop through the
                        // ! drain below instead.
                        let next = match create_pipe(&self.name, &self.sddl, false) {
                            Ok(next) => next,
                            Err(_) => { failure = Some(FAILURE); break; }
                        };
                        let pipe = std::mem::replace(&mut self.pipe, next);
                        let future = serve_connection(
                            pipe,
                            dispatch,
                            Arc::clone(&self.allowed),
                            stop.clone(),
                            Arc::clone(&capacity),
                            reader,
                            self.authorize,
                        );
                        active.spawn(future);
                    }
                    Err(error) if peer_disconnected(&error) => {
                        if self.renew().is_err() { failure = Some(FAILURE); break; }
                    }
                    Err(_) => { failure = Some(FAILURE); break; }
                },
                result = active.join_next(), if !active.is_empty() => {
                    match result {
                        Some(Ok(Err(error))) => { failure = Some(error); break; }
                        Some(Err(_)) => { failure = Some(FAILURE); break; }
                        _ => {}
                    }
                }
                _ = stop.changed() => break,
            }
        }
        while let Some(result) = active.join_next().await {
            match result {
                Ok(Err(error)) => {
                    failure.get_or_insert(error);
                }
                Err(_) => {
                    failure.get_or_insert(FAILURE);
                }
                _ => {}
            }
        }
        failure.map_or(Ok(()), Err)
    }

    fn renew(&mut self) -> Result<(), &'static str> {
        // ! Don't reuse Tokio/Mio's buffered I/O across clients. Create the replacement while the
        // ! old instance still owns the pipe name, then close the old connected instance without
        // ! letting it listen again. The replacement stays available while callbacks run.
        let next = create_pipe(&self.name, &self.sddl, false)?;
        self.pipe = next;
        Ok(())
    }
}
