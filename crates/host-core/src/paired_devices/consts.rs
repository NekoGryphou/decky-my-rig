/// Paired devices per PC. If you change it, also change `MaxPairedDevices` in
/// apps/windows-app/src/MyRigConfig.Core/LocalControl/LocalControlConstants.cs.
pub const MAX_PAIRED_DEVICES: usize = 16;
pub(super) const SCHEMA_VERSION: u32 = 3;
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;
