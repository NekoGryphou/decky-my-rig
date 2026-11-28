use hyper::Method;
use myrig_protocol::lan::{PAIR_INFO_PATH, PAIR_REQUEST_PATH};
use std::time::Duration;

pub(super) const MAX_HEADER_BYTES: usize = 16_384;
pub(super) const MAX_BODY_BYTES: usize = 16_384;
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// Status and shutdown calls: a PC that doesn't answer in this time is offline.
pub(super) const STATUS_DEADLINE: Duration = Duration::from_secs(3);
pub(super) const ALPN: &[u8] = b"http/1.1";
/// Repeated framing headers would make the response length ambiguous.
pub(super) const FRAMING_HEADERS: [&str; 3] =
    ["content-type", "content-length", "transfer-encoding"];
/// The only requests we send before the host certificate is trusted, as
/// (method, path, has body): the OPAQUE info request, which the exchange
/// authenticates afterwards, and the pairing-code request, which carries no
/// secret.
pub(super) const UNVERIFIED_ROUTES: [(Method, &str, bool); 2] = [
    (Method::GET, PAIR_INFO_PATH, false),
    (Method::POST, PAIR_REQUEST_PATH, true),
];
