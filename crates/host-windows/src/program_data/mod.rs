//! The machine-wide application data folder (normally C:\ProgramData).
mod native;

pub(crate) use native::program_data;
