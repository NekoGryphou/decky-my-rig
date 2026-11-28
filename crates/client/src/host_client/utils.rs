use super::{
    consts::{FRAMING_HEADERS, MAX_BODY_BYTES, MAX_HEADER_BYTES},
    errors::HostError,
    types::HttpReply,
};
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::{
    Method, Request, Response,
    body::{Bytes, Incoming},
    header::{
        ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, HOST,
        HeaderValue, TRANSFER_ENCODING,
    },
};
use hyper_util::rt::TokioIo;
use myrig_protocol::{
    checked_json::CheckedJson,
    lan::{PROTOCOL_MAJOR, ProtocolVersion},
};
use serde_json::{Map, Value};
use tokio::io::{AsyncRead, AsyncWrite};
use zeroize::Zeroizing;

pub(super) fn build_request(
    method: Method,
    path: &str,
    host: &str,
    token: Option<&str>,
    body: Option<Vec<u8>>,
) -> Result<Request<Full<Bytes>>, HostError> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(HOST, host)
        .header(ACCEPT, "application/json")
        .header("x-myrig-protocol-major", PROTOCOL_MAJOR.to_string())
        .header(CONNECTION, "close");
    if let Some(token) = token {
        // * We can't wipe hyper's copy of the header, since hyper owns those bytes.
        let text = Zeroizing::new(format!("Bearer {token}"));
        let mut value = HeaderValue::from_str(&text).map_err(|_| HostError::Invalid)?;
        value.set_sensitive(true);
        builder = builder.header(AUTHORIZATION, value);
    }
    if body.is_some() {
        builder = builder.header(CONTENT_TYPE, "application/json");
    }
    builder
        .body(Full::new(Bytes::from(body.unwrap_or_default())))
        .map_err(|_| HostError::Invalid)
}

/// Sends one request on an open stream and reads the reply, up to the size
/// limit. The connection is driven inside this future, so nothing outlives the
/// call.
pub(super) async fn send<S>(
    stream: S,
    request: Request<Full<Bytes>>,
) -> Result<HttpReply, HostError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, connection) = hyper::client::conn::http1::Builder::new()
        .max_buf_size(MAX_HEADER_BYTES)
        .handshake(TokioIo::new(stream))
        .await
        .map_err(|_| HostError::Connection)?;
    let exchange = async move {
        let response = sender.send_request(request).await.map_err(classify)?;
        read_reply(response).await
    };
    tokio::pin!(exchange, connection);
    tokio::select! {
        reply = &mut exchange => reply,
        _ = &mut connection => exchange.await,
    }
}

fn classify(error: hyper::Error) -> HostError {
    if error.is_parse() || error.is_parse_too_large() || error.is_parse_status() {
        HostError::Invalid
    } else {
        HostError::Connection
    }
}

async fn read_reply(response: Response<Incoming>) -> Result<HttpReply, HostError> {
    let headers = response.headers();
    if FRAMING_HEADERS
        .iter()
        .any(|name| headers.get_all(*name).iter().count() > 1)
        || headers.contains_key(TRANSFER_ENCODING)
    {
        return Err(HostError::Invalid);
    }
    if let Some(length) = headers.get(CONTENT_LENGTH) {
        let length = length
            .to_str()
            .ok()
            .filter(|text| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|text| text.parse::<usize>().ok())
            .ok_or(HostError::Invalid)?;
        if length > MAX_BODY_BYTES {
            return Err(HostError::Invalid);
        }
    }
    let text = |name| {
        headers
            .get(name)
            .and_then(|value: &HeaderValue| value.to_str().ok())
            .map(str::to_owned)
    };
    let (content_type, cache_control) = (text(CONTENT_TYPE), text(CACHE_CONTROL));
    let status = response.status().as_u16();
    let body = Limited::new(response.into_body(), MAX_BODY_BYTES)
        .collect()
        .await
        .map_err(|error| {
            if error.is::<LengthLimitError>() {
                HostError::Invalid
            } else {
                HostError::Connection
            }
        })?
        .to_bytes();
    Ok(HttpReply {
        status,
        content_type,
        cache_control,
        body: body.to_vec(),
    })
}

/// The reply's JSON object, if the status is exactly the one the route expects
/// and the protocol major version matches.
pub fn parse_reply(reply: &HttpReply, expected: u16) -> Result<Value, HostError> {
    match reply.status {
        401 => return Err(HostError::Unauthorized),
        404 | 500.. => return Err(HostError::Connection),
        status if status != expected => return Err(HostError::Invalid),
        _ => {}
    }
    let media_type = reply
        .content_type
        .as_deref()
        .and_then(|value| value.split(';').next())
        .unwrap_or("");
    if !media_type.trim().eq_ignore_ascii_case("application/json") {
        return Err(HostError::Invalid);
    }
    let value = if reply.body.is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_slice::<CheckedJson>(&reply.body)
            .map_err(|_| HostError::Invalid)?
            .0
    };
    let version: ProtocolVersion = value
        .get("protocol_version")
        .and_then(|version| serde_json::from_value(version.clone()).ok())
        .ok_or(HostError::Invalid)?;
    if version.major != PROTOCOL_MAJOR {
        return Err(HostError::UnsupportedProtocol);
    }
    Ok(value)
}
