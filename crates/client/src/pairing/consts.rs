use std::time::Duration;

pub(super) const PAIR_DEADLINE: Duration = Duration::from_secs(10);
pub(super) const CODE_REQUEST_DEADLINE: Duration = Duration::from_secs(5);
/// How long we wait for the PC to revoke an unsaved pairing we're dropping.
/// Finish gets its own budget: the memory-hard step before it can use most of
/// the pairing deadline, and the PC saves the device as soon as it answers.
pub(super) const FINISH_DEADLINE: Duration = Duration::from_secs(5);
pub(super) const REVOKE_DEADLINE: Duration = Duration::from_secs(3);
/// 202 means the PC shows a new code, 409 means a code is already showing.
pub(super) const CODE_SHOWN_STATUSES: [u16; 2] = [202, 409];
pub(super) const BOOTSTRAP_VERSION: u64 = 1;
pub(super) const FALLBACK_DEVICE_NAME: &str = "My device";
/// The PC holds on to its code briefly while it handles another request.
pub(super) const BUSY_STATUS: u16 = 503;
pub(super) const BUSY_RETRY: Duration = Duration::from_millis(250);
/// Time for the store write that follows credential issuance, or the revoke if
/// that write fails.
const STORE_WRITE_MARGIN: Duration = Duration::from_secs(2);
/// The longest a pairing needs after the PC has started issuing a credential:
/// the finish request, then the store write or the revoke. The agent waits this
/// long for it when stdin closes.
pub(crate) const CREDENTIAL_BUDGET: Duration = Duration::from_secs(
    FINISH_DEADLINE.as_secs() + REVOKE_DEADLINE.as_secs() + STORE_WRITE_MARGIN.as_secs(),
);
