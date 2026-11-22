use std::{
    io::{BufRead, Write},
    path::PathBuf,
};

/// Where a command reads answers and writes its results and errors.
pub struct Terminal<'a> {
    pub input: &'a mut dyn BufRead,
    pub output: &'a mut dyn Write,
    pub errors: &'a mut dyn Write,
}

pub(super) enum Command {
    List,
    Status(Option<String>),
    Wake(String),
    Stop { name: String, yes: bool },
    Pair { host: String, port: String },
    Rename { name: String, new_name: String },
    Remove { name: String, yes: bool },
    Agent { store: PathBuf },
}

/// A failed command: its exit code and the message for the user.
pub(super) struct Failure {
    pub(super) code: u8,
    pub(super) message: String,
}
