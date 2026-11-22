//! LAN API v1 bodies and routes shared by the host service and the client.
mod consts;
mod types;
mod utils;

pub use consts::{
    PAIR_FINISH_PATH, PAIR_INFO_PATH, PAIR_REQUEST_PATH, PAIR_START_PATH, PAIRING_PATH,
    PROTOCOL_MAJOR, PROTOCOL_MINOR, SHUTDOWN_PATH, SHUTDOWN_TEST_PATH, STATUS_PATH,
};
pub use types::{
    HostState, PairCodeRequest, PairFinishReply, PairFinishRequest, PairInfoReply, PairStartReply,
    PairStartRequest, ProtocolVersion, ShutdownReply, ShutdownRequest, StatusReply, UnpairReply,
    Versioned,
};

#[cfg(test)]
mod tests;
