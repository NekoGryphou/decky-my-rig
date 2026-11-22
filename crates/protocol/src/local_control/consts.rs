use std::time::Duration;

pub const CONTROL_SOCKET_NAME: &str = "MyRigHost.Control.v1";
pub const REQUEST_LIMIT: usize = 1024;
pub const RESPONSE_LIMIT: usize = 8192;
/// Ports below this are reserved for system services, so `set_port` refuses
/// them.
pub const MIN_SETTABLE_PORT: u16 = 1024;
/// How long a pairing code stays valid. A reply promising longer is refused.
/// The host's session enforces this (`LIFETIME` in `myrig_pairing_crypto`,
/// which a host-core test keeps in sync).
pub const PAIRING_CODE_LIFETIME: Duration = Duration::from_secs(120);
