use super::{
    super::consts::STATUS_DEADLINE,
    types::{Environment, LiveEnvironment},
};
use crate::{
    host_client::{HostClient, HostError, ShutdownResult, Status},
    mac_detection::detect_mac,
    network::{WakeError, wake},
    shared::PublicError,
    store::PcConfig,
};
use std::{net::Ipv4Addr, time::Instant};
use uuid::Uuid;

fn client(pc: &PcConfig) -> HostClient {
    HostClient::new(
        pc.host,
        pc.port,
        Some(pc.authentication.token.to_string()),
        Some(pc.authentication.tls_pin),
        STATUS_DEADLINE,
    )
}

impl Environment for LiveEnvironment {
    fn now(&self) -> Instant {
        Instant::now()
    }
    async fn status(&self, pc: &PcConfig) -> Result<Status, HostError> {
        client(pc).status().await
    }
    async fn shutdown(&self, pc: &PcConfig, request_id: Uuid) -> Result<ShutdownResult, HostError> {
        client(pc).shutdown(request_id).await
    }
    async fn unpair(&self, pc: &PcConfig) -> Result<(), HostError> {
        client(pc).unpair().await
    }
    async fn wake(&self, pc: &PcConfig) -> Result<(), WakeError> {
        wake(self.0.as_ref(), &pc.mac, pc.host).await
    }
    fn detect_mac(&self, host: Ipv4Addr) -> Result<String, PublicError> {
        detect_mac(self.0.as_ref(), host)
    }
}
