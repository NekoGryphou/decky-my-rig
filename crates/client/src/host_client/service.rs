use super::{
    consts::{ALPN, CONNECT_TIMEOUT, UNVERIFIED_ROUTES},
    errors::HostError,
    pinning::PinVerifier,
    types::{HostClient, HttpReply, ShutdownResult, Status},
    utils::{build_request, parse_reply, send},
};
use crate::shared::normalize_uuid;
use hyper::Method;
use myrig_protocol::lan::{
    HostState, PAIRING_PATH, SHUTDOWN_PATH, STATUS_PATH, ShutdownReply, ShutdownRequest,
    StatusReply, UnpairReply,
};
use rustls::{ClientConfig, client::Resumption, crypto::ring, pki_types::ServerName};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};
use tokio::{net::TcpStream, time::timeout};
use tokio_rustls::TlsConnector;
use uuid::Uuid;
use zeroize::Zeroizing;

impl HostClient {
    /// `deadline` bounds each whole request: connect, handshake and reply.
    pub fn new(
        host: Ipv4Addr,
        port: u16,
        token: Option<String>,
        pin: Option<[u8; 32]>,
        deadline: Duration,
    ) -> Self {
        Self {
            address: SocketAddrV4::new(host, port),
            token: token.map(Zeroizing::new),
            pin,
            deadline,
        }
    }

    pub async fn status(&self) -> Result<Status, HostError> {
        // ? Parsing only accepts online and shutting_down. Any other state would
        //   leave the device polling a PC it can't make sense of.
        let reply: StatusReply = self.call(Method::GET, STATUS_PATH, None, 200).await?;
        Ok(Status {
            host_id: normalize_uuid(&reply.host_id).map_err(|_| HostError::Invalid)?,
            state: reply.state,
        })
    }

    /// Sends one shutdown request. If the result is unclear, don't retry it.
    pub async fn shutdown(&self, request_id: Uuid) -> Result<ShutdownResult, HostError> {
        let request_id = request_id.to_string();
        let body = serde_json::to_vec(&ShutdownRequest {
            request_id: request_id.clone(),
        })
        .map_err(|_| HostError::Invalid)?;
        let reply: ShutdownReply = self
            .call(Method::POST, SHUTDOWN_PATH, Some(body), 202)
            .await?;
        if reply.request_id != request_id || reply.state != HostState::ShuttingDown {
            return Err(HostError::Invalid);
        }
        match reply.test {
            None => Ok(ShutdownResult { test: false }),
            Some(true) => Ok(ShutdownResult { test: true }),
            Some(false) => Err(HostError::Invalid),
        }
    }

    /// Asks the PC to forget this client's credential. `revoked` is only false
    /// when another removal got there first, which ends the same way.
    pub async fn unpair(&self) -> Result<(), HostError> {
        let _: UnpairReply = self.call(Method::DELETE, PAIRING_PATH, None, 200).await?;
        Ok(())
    }

    /// Sends one bootstrap request to a host we don't trust yet, and returns
    /// the certificate it presented so the caller can authenticate it.
    pub async fn request_unverified(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<(HttpReply, Vec<u8>), HostError> {
        let allowed = UNVERIFIED_ROUTES.iter().any(|(allowed, route, has_body)| {
            *allowed == method && *route == path && *has_body == body.is_some()
        });
        if self.token.is_some() || self.pin.is_some() || !allowed {
            return Err(HostError::Invalid);
        }
        self.exchange(method, path, body).await
    }

    /// Sends one request to the host if its certificate matches the pin, and
    /// returns the raw reply for routes where the caller reads the status
    /// itself.
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<HttpReply, HostError> {
        if self.pin.is_none() {
            return Err(HostError::Invalid);
        }
        Ok(self.exchange(method, path, body).await?.0)
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
        expected: u16,
    ) -> Result<T, HostError> {
        let reply = self.request(method, path, body).await?;
        let value: Value = parse_reply(&reply, expected)?;
        serde_json::from_value(value).map_err(|_| HostError::Invalid)
    }

    async fn exchange(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<(HttpReply, Vec<u8>), HostError> {
        timeout(
            self.deadline,
            self.exchange_within_deadline(method, path, body),
        )
        .await
        .map_err(|_| HostError::Timeout)?
    }

    async fn exchange_within_deadline(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> Result<(HttpReply, Vec<u8>), HostError> {
        let tcp = timeout(CONNECT_TIMEOUT, TcpStream::connect(self.address))
            .await
            .map_err(|_| HostError::Timeout)?
            .map_err(|_| HostError::Connection)?;
        let provider = Arc::new(ring::default_provider());
        let verifier = Arc::new(PinVerifier::new(self.pin, provider.clone()));
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| HostError::Invalid)?
            .dangerous()
            .with_custom_certificate_verifier(verifier.clone())
            .with_no_client_auth();
        config.alpn_protocols = vec![ALPN.to_vec()];
        // ! Session resumption would skip the pin check, so every request does a
        //   full handshake.
        config.resumption = Resumption::disabled();
        let server_name = ServerName::IpAddress((*self.address.ip()).into());
        let stream = match TlsConnector::from(Arc::new(config))
            .connect(server_name, tcp)
            .await
        {
            Ok(stream) => stream,
            Err(_) if verifier.mismatched() => return Err(HostError::PinMismatch),
            Err(_) => return Err(HostError::Connection),
        };
        let certificate = verifier.certificate().ok_or(HostError::Unauthorized)?;
        if stream.get_ref().1.alpn_protocol() != Some(ALPN) {
            return Err(HostError::Invalid);
        }
        let request = build_request(
            method,
            path,
            &self.address.to_string(),
            self.token.as_ref().map(|token| token.as_str()),
            body,
        )?;
        Ok((send(stream, request).await?, certificate))
    }
}
