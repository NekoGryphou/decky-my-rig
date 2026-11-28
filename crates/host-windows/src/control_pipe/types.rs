use std::{os::windows::io::OwnedHandle, sync::Arc, thread};
use tokio::{net::windows::named_pipe::NamedPipeServer, sync::oneshot};
use zeroize::Zeroizing;

pub(super) type ControlCallback<Context> =
    Arc<dyn Fn(&[u8], Context) -> Zeroizing<Vec<u8>> + Send + Sync>;

pub(super) struct Dispatch<Context> {
    pub(super) callback: ControlCallback<Context>,
    pub(super) context: Context,
}

pub(super) struct Worker {
    pub(super) thread: Option<thread::JoinHandle<()>>,
    pub(super) response: oneshot::Receiver<Result<Zeroizing<Vec<u8>>, &'static str>>,
}
/// Whether the impersonated caller of a pipe handle may manage the host.
pub(super) type Authorize = fn(&OwnedHandle, &Allowed) -> Result<bool, &'static str>;

/// Who may use the pipe. `always` are SIDs whose token membership is enough.
/// A recorded user's own SID counts only while that user is still in the
/// managers group, which is read again for every connection.
pub(super) struct Allowed {
    pub(super) always: Vec<Vec<u32>>,
    pub(super) recorded_users: Vec<RecordedUser>,
}

pub(super) struct RecordedUser {
    pub(super) text: String,
    pub(super) words: Vec<u32>,
}

pub struct ControlPipe {
    pub(super) pipe: NamedPipeServer,
    pub(super) name: String,
    pub(super) sddl: String,
    pub(super) allowed: Arc<Allowed>,
    pub(super) authorize: Authorize,
}

pub(super) struct Capacity {
    pub(super) normal: Arc<tokio::sync::Semaphore>,
    pub(super) events: Arc<tokio::sync::Semaphore>,
    pub(super) reading: Arc<tokio::sync::Semaphore>,
}
