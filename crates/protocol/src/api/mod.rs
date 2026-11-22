//! What the client reports to its user interfaces: the shapes in the Decky
//! plugin's `api.types.ts`, also printed by the command line.
mod types;

pub use types::{
    DeleteResult, PairingDraftResult, PcIssue, PcSettings, PcSnapshot, PowerState, SnapshotFrame,
    TestStops,
};

#[cfg(test)]
mod tests;
