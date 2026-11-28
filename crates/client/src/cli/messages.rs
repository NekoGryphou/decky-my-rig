pub(super) const USAGE_TEXT: &str = "Usage:
  myrig list
  myrig status [NAME]
  myrig wake NAME
  myrig stop NAME [--yes]
  myrig pair HOST [--port PORT]
  myrig rename NAME NEW_NAME
  myrig remove NAME [--yes]
  myrig agent --store DIRECTORY
NAME is a saved PC's name or ID.";
pub(super) const NO_PCS: &str = "No saved PCs.";
pub(super) const CODE_PROMPT: &str = "Enter the pairing code shown on the PC: ";
pub(super) const MAC_PROMPT: &str = "MAC address (for Wake-on-LAN): ";
pub(super) const CANCELLED: &str = "Cancelled.";
pub(super) const ONLINE: &str = "online";
pub(super) const SHUTTING_DOWN: &str = "shutting down";
pub(super) const OFFLINE: &str = "offline";
pub(super) const NOT_PAIRED: &str = "not paired any more; pair again";
pub(super) const IDENTITY_CHANGED: &str = "identity changed; pair again";
pub(super) const INCOMPATIBLE: &str = "incompatible; update My Rig";
pub(super) const UNREACHABLE_PREFIX: &str = "PC is unreachable";
pub(super) const SHUTDOWN_STUCK: &str =
    "A save is still waiting for another process to release the PC configuration.";
