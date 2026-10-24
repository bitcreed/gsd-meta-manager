//! The inference upstream and its test seam (28.1-01, RESEARCH Pattern 2).
//!
//! Production dials `api.anthropic.com:443` and speaks TLS (rustls with the ring
//! provider and the compiled-in Mozilla roots). A test hands the server
//! [`Upstream::Plain`], a loopback `SocketAddr` of an in-process stub, so the whole
//! request path runs in CI with no network. The seam is a Rust value: it is never
//! read from the environment, so nothing the driver can set re-routes the token.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// How long dialling the upstream (TCP, and TLS on top of it) may take in total.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Where inference requests go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upstream {
    /// TLS to `host:port`; `host` is also the SNI name and the name the
    /// certificate must match.
    Tls { host: String, port: u16 },
    /// Plain TCP to `addr`. The test seam; never built from the environment.
    Plain(SocketAddr),
}

impl Upstream {
    /// The production upstream: `api.anthropic.com:443`.
    pub fn anthropic() -> Self {
        Upstream::Tls {
            host: super::INFERENCE_HOST.to_owned(),
            port: 443,
        }
    }
}

/// A connected upstream byte stream, TLS or plain.
pub trait UpstreamIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> UpstreamIo for T {}

/// A boxed [`UpstreamIo`].
pub type UpstreamStream = Box<dyn UpstreamIo>;

/// An [`Upstream`] with its TLS client configuration built once, so each request
/// only pays for the handshake.
pub struct Connector {
    upstream: Upstream,
    tls: Option<TlsConnector>,
}

impl Connector {
    /// Prepare to dial `upstream`. Builds the rustls configuration for a TLS
    /// upstream: ring provider, the webpki root set, and ALPN pinned to
    /// `http/1.1` (RESEARCH Pitfall 5: the proxy writes HTTP/1.1, so an `h2`
    /// negotiation would be a protocol error).
    pub fn new(upstream: Upstream) -> io::Result<Self> {
        let tls = match &upstream {
            Upstream::Plain(_) => None,
            Upstream::Tls { .. } => {
                let mut roots = rustls::RootCertStore::empty();
                roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                let provider = Arc::new(rustls::crypto::ring::default_provider());
                let mut config = rustls::ClientConfig::builder_with_provider(provider)
                    .with_safe_default_protocol_versions()
                    .map_err(io::Error::other)?
                    .with_root_certificates(roots)
                    .with_no_client_auth();
                config.alpn_protocols = vec![b"http/1.1".to_vec()];
                Some(TlsConnector::from(Arc::new(config)))
            }
        };
        Ok(Connector { upstream, tls })
    }

    /// Dial the upstream within [`CONNECT_TIMEOUT`].
    pub async fn connect(&self) -> io::Result<UpstreamStream> {
        match tokio::time::timeout(CONNECT_TIMEOUT, self.dial()).await {
            Ok(result) => result,
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "upstream connect timed out",
            )),
        }
    }

    async fn dial(&self) -> io::Result<UpstreamStream> {
        match (&self.upstream, &self.tls) {
            (Upstream::Plain(addr), _) => Ok(Box::new(TcpStream::connect(addr).await?)),
            (Upstream::Tls { host, port }, Some(tls)) => {
                let name = rustls::pki_types::ServerName::try_from(host.clone())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                let tcp = TcpStream::connect((host.as_str(), *port)).await?;
                Ok(Box::new(tls.connect(name, tcp).await?))
            }
            (Upstream::Tls { .. }, None) => Err(io::Error::other("TLS upstream without a config")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_production_upstream_is_the_inference_host_on_443() {
        assert_eq!(
            Upstream::anthropic(),
            Upstream::Tls {
                host: "api.anthropic.com".to_owned(),
                port: 443
            }
        );
    }

    #[test]
    fn a_tls_connector_builds_with_the_ring_provider() {
        Connector::new(Upstream::anthropic()).expect("a rustls config");
    }
}
