use crate::{
    host_client::{HostError, ShutdownResult, Status},
    network::WakeError,
    platform::Platform,
    shared::PublicError,
    store::PcConfig,
};
use std::{future::Future, net::Ipv4Addr, sync::Arc, time::Instant};
use uuid::Uuid;

pub trait Environment: Send + Sync + 'static {
    fn now(&self) -> Instant;
    fn status(&self, pc: &PcConfig) -> impl Future<Output = Result<Status, HostError>> + Send;
    fn shutdown(
        &self,
        pc: &PcConfig,
        request_id: Uuid,
    ) -> impl Future<Output = Result<ShutdownResult, HostError>> + Send;
    fn unpair(&self, pc: &PcConfig) -> impl Future<Output = Result<(), HostError>> + Send;
    fn wake(&self, pc: &PcConfig) -> impl Future<Output = Result<(), WakeError>> + Send;
    fn detect_mac(&self, host: Ipv4Addr) -> Result<String, PublicError>;
}

/// The real network: pinned host requests with a 3 s deadline, Wake-on-LAN, and
/// this device's neighbor table, all through `platform`.
pub struct LiveEnvironment(pub Arc<dyn Platform>);
