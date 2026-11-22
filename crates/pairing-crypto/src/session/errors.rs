#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Closed,
    Busy,
    Limit,
    StaleAttempt,
    Crypto,
}
