use crate::{
    pairing::Paired,
    store::{PcConfig, Store},
};
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex, atomic::AtomicBool},
};
use tokio::task::JoinSet;
use uuid::Uuid;

pub struct Drafts {
    pub(super) store: Arc<Store>,
    /// This device's name, as the PC shows it while pairing.
    pub(super) device: String,
    pub(super) state: Mutex<DraftState>,
    /// Set while an exchange or save may hold a credential the PC issued that
    /// isn't stored or revoked yet. Only one runs at a time.
    pub(super) holding: AtomicBool,
    /// Saves still writing or revoking, which `close` waits for.
    pub(super) settling: Mutex<JoinSet<()>>,
}

/// Identifies one reserved draft. It goes stale once that draft is canceled or
/// replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reservation(pub(super) u64);

#[derive(Default)]
pub(super) struct DraftState {
    /// The draft the user is working on, if any.
    pub(super) draft: Option<Draft>,
    /// The draft whose exchange or save is running. Only one at a time.
    pub(super) active: Option<u64>,
    pub(super) next: u64,
    pub(super) closed: bool,
}

pub(super) struct Draft {
    pub(super) generation: u64,
    pub(super) id: Uuid,
    pub(super) host: Ipv4Addr,
    pub(super) port: u16,
    /// The saved PC being re-paired, as we last saw it.
    pub(super) original: Option<PcConfig>,
    pub(super) started: bool,
    pub(super) paired: Option<Paired>,
    /// True when the new credential is already in the store: a re-pair answered
    /// by the same certificate, which revoked the old one.
    pub(super) kept: bool,
}

/// Frees the exchange slot when an exchange ends or its future is dropped, and
/// throws the draft away unless the exchange published it.
pub(super) struct Release<'a> {
    pub(super) drafts: &'a Drafts,
    pub(super) generation: u64,
    pub(super) published: bool,
}
