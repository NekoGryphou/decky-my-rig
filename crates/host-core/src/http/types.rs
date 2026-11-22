use http_body_util::Full;
use hyper::{Response, body::Bytes};

pub(super) type Reply = Response<Full<Bytes>>;

/// Why a request body couldn't be read.
pub(super) enum BodyError {
    TooLarge,
    Invalid,
}
