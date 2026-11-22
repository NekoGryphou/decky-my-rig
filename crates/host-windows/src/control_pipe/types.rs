use std::{os::windows::io::OwnedHandle, sync::Arc, thread};
use tokio::{net::windows::named_pipe::NamedPipeServer, sync::oneshot};

pub(super) type ControlCallback<Context> = Arc<dyn Fn(&[u8], Context) -> Vec<u8> + Send + Sync>;

pub(super) struct Dispatch<Context> {
    pub(super) callback: ControlCallback<Context>,
    pub(super) context: Context,
}

pub(super) struct Worker {
    pub(super) thread: Option<thread::JoinHandle<()>>,
    pub(super) response: oneshot::Receiver<Result<Vec<u8>, &'static str>>,
}
/// Whether the impersonated caller of a pipe handle may manage the host.
pub(super) type Authorize = fn(&OwnedHandle, &[Vec<u32>]) -> Result<bool, &'static str>;

pub struct ControlPipe {
    pub(super) pipe: NamedPipeServer,
    pub(super) name: String,
    pub(super) sddl: String,
    pub(super) allowed_sids: Arc<Vec<Vec<u32>>>,
    pub(super) authorize: Authorize,
}

pub(super) struct Capacity {
    pub(super) normal: Arc<tokio::sync::Semaphore>,
    pub(super) events: Arc<tokio::sync::Semaphore>,
    pub(super) reading: Arc<tokio::sync::Semaphore>,
}
