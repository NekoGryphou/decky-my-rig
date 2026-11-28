// ! The rights a client gets: read and write data, extended attributes and attributes, READ_CONTROL
// ! and SYNCHRONIZE. Not FILE_CREATE_PIPE_INSTANCE (4), DELETE, WRITE_DAC or WRITE_OWNER.
pub(crate) const CONTROL_CLIENT_ACCESS: u32 = 0x0012019b;
/// How long a client waits for a busy pipe to free an instance. The service
/// opens a new instance as soon as one connects, so a longer wait means it is
/// stuck or stopping. Callers block for this long at most.
pub(crate) const PIPE_BUSY_WAIT_MS: u32 = 2000;
