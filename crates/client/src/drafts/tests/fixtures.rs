use crate::drafts::Drafts;
use crate::host_client::fixtures::{RealHost, real_host_with, token};
use crate::shared::PublicError;
use crate::store::{Authentication, PcConfig, PcInput, Store};
use myrig_protocol::api::PairingDraftResult;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

pub(super) const CODE: &str = "12345678";

pub(super) fn found(_: Ipv4Addr) -> Result<String, PublicError> {
    Ok("02:00:00:00:00:09".to_owned())
}

pub(super) fn missing(_: Ipv4Addr) -> Result<String, PublicError> {
    Err(PublicError(
        "MAC detection failed. Enter the MAC address manually.",
    ))
}

pub(super) struct Fixture {
    pub(super) _directory: TempDir,
    pub(super) store: Arc<Store>,
    pub(super) drafts: Drafts,
    pub(super) host: RealHost,
}

pub(super) async fn fixture(code: Option<&str>) -> Fixture {
    let directory = TempDir::new().unwrap();
    let store =
        Arc::new(Store::open(directory.path(), crate::platform::fixtures::platform()).unwrap());
    Fixture {
        drafts: Drafts::new(store.clone(), "test-device".to_owned()),
        store,
        _directory: directory,
        host: real_host_with(true, code).await,
    }
}

impl Fixture {
    pub(super) fn port(&self) -> String {
        self.host.address.port().to_string()
    }
    /// A PC saved with the host's pin and the registry's token 1.
    pub(super) fn saved(&self) -> PcConfig {
        self.store
            .add(
                PcInput {
                    name: "Gaming PC".into(),
                    host: "127.0.0.1".into(),
                    port: Some(self.port()),
                    mac: "02:00:00:00:00:01".into(),
                },
                Authentication {
                    host_id: Uuid::parse_str(crate::host_client::fixtures::HOST_ID).unwrap(),
                    token: token(1).into(),
                    tls_pin: self.host.pin,
                },
            )
            .unwrap()
    }
    pub(super) async fn begin(
        &self,
        draft: Uuid,
        code: &str,
        original: Option<&PcConfig>,
    ) -> Result<PairingDraftResult, &'static str> {
        let original = original.map(|pc| pc.id.to_string());
        self.drafts
            .begin(
                &draft.to_string(),
                "127.0.0.1",
                &self.port(),
                code,
                original.as_deref(),
                found,
            )
            .await
            .map_err(|error| error.0)
    }
}
