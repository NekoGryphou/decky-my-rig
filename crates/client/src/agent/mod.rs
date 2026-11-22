//! `myrig agent` talks to the Decky plugin in JSON lines over stdin and stdout.
//! A request `{"id","method","params"}` gets back `{"id","result"}` or
//! `{"id","error"}`, and every new snapshot is pushed as
//! `{"event":"snapshot","frame"}`. The agent exits when stdin closes.
mod consts;
mod messages;
mod service;
mod types;
mod utils;

pub use service::serve;

#[cfg(test)]
mod tests;
