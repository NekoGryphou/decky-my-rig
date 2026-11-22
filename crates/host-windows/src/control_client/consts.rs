// ! The rights a client gets: read and write data, extended attributes and
//   attributes, READ_CONTROL and SYNCHRONIZE. Not FILE_CREATE_PIPE_INSTANCE (4),
//   DELETE, WRITE_DAC or WRITE_OWNER.
pub(crate) const CONTROL_CLIENT_ACCESS: u32 = 0x0012019b;
