//! The egress decisions, all pure (28.1-01, RESEARCH Pattern 2).
//!
//! Every function here takes parsed data and returns a verdict. None reads a
//! file, resolves a name or touches a socket, so the whole policy is table-tested
//! in CI the way `sandbox::bwrap_argv` is. The I/O modules only call these and act
//! on the answer; a request is judged here BEFORE any upstream connection exists.

/// One request header as read off the wire: the name as sent (compare it with
/// [`header_is`]) and the raw value bytes.
pub type Header = (String, Vec<u8>);

/// Request headers that are never forwarded upstream (RESEARCH Q1 rule 6).
///
/// The first group is the client's own credentials: the launcher injects the one
/// `Authorization` the upstream sees, and a driver-supplied `x-api-key` or
/// `cookie` must never ride alongside it. The second group is hop-by-hop or
/// framing state the proxy re-creates itself (`Host`, `Content-Length`,
/// `Connection: close`). Every `proxy-*` header is dropped as well
/// ([`is_dropped_request_header`]).
pub const DROP_REQUEST_HEADERS: &[&str] = &[
    "authorization",
    "x-api-key",
    "cookie",
    "host",
    "connection",
    "keep-alive",
    "te",
    "trailer",
    "upgrade",
    "transfer-encoding",
    "content-length",
    "expect",
];

/// What a request-line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// `POST /v1/messages` or `POST /v1/messages/count_tokens`, origin-form.
    /// `query` is the text after the first `?`, verbatim (D-05: any query).
    Inference { path: String, query: Option<String> },
    /// `CONNECT host:port`, authority-form. The host keeps its brackets when it
    /// was an IPv6 literal, so [`connect_verdict`] can name it as an IP literal.
    Connect { host: String, port: u16 },
    /// Anything else: absolute-form, asterisk-form, another path or method.
    Refuse(String),
}

/// The paths the inference reverse proxy forwards (D-05).
const INFERENCE_PATHS: &[&str] = &["/v1/messages", "/v1/messages/count_tokens"];

/// Classify a request-line.
///
/// Origin-form only on the inference side: an absolute-form target
/// (`POST http://api.anthropic.com/v1/messages`) is what an `HTTP_PROXY` client
/// sends for plain HTTP, and a proxy that strips the scheme and host forwards it
/// to the inference upstream WITH the token (RESEARCH Pitfall 2). It is refused
/// here, together with every target that is not plain visible ASCII, so what is
/// later written into the upstream request line cannot carry a control byte.
pub fn classify_target(method: &str, target: &str) -> Target {
    if method == "CONNECT" {
        return match parse_authority(target) {
            Some((host, port)) => Target::Connect { host, port },
            None => Target::Refuse("CONNECT target is not host:port".to_owned()),
        };
    }
    if !target.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        return Target::Refuse("request target is not visible ASCII".to_owned());
    }
    if !target.starts_with('/') {
        return Target::Refuse(
            "only origin-form request targets are served (no absolute-form, no '*')".to_owned(),
        );
    }
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, Some(query.to_owned())),
        None => (target, None),
    };
    if !INFERENCE_PATHS.contains(&path) {
        return Target::Refuse("path is not an inference endpoint".to_owned());
    }
    if method != "POST" {
        return Target::Refuse("inference endpoints accept POST only".to_owned());
    }
    Target::Inference {
        path: path.to_owned(),
        query,
    }
}

/// `host:port` or `[v6]:port` to `(host, port)`. The port must be decimal digits
/// that fit a `u16`; the host must be non-empty visible ASCII with no `/`, `@`
/// or `?` (an authority has no userinfo, path or query).
fn parse_authority(target: &str) -> Option<(String, u16)> {
    let (host, port) = target.rsplit_once(':')?;
    if host.is_empty() || port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !host.bytes().all(|b| (0x21..=0x7e).contains(&b))
        || host.bytes().any(|b| matches!(b, b'/' | b'@' | b'?' | b'#'))
    {
        return None;
    }
    Some((host.to_owned(), port.parse().ok()?))
}

/// Whether `header` is the header called `name`; `name` is lowercase.
pub fn header_is(header: &Header, name: &str) -> bool {
    header.0.eq_ignore_ascii_case(name)
}

/// Whether a request header named `name` must not reach the upstream.
pub fn is_dropped_request_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("proxy-") || DROP_REQUEST_HEADERS.contains(&lower.as_str())
}

/// The client's headers that survive to the upstream: everything except
/// [`DROP_REQUEST_HEADERS`] and `proxy-*`, in their original order and spelling.
/// `anthropic-*`, `x-stainless-*`, `user-agent`, `accept*` and `content-type`
/// pass through untouched.
pub fn forwarded_headers(headers: &[Header]) -> Vec<Header> {
    headers
        .iter()
        .filter(|(name, _)| !is_dropped_request_header(name))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(method: &str, target: &str) -> bool {
        matches!(classify_target(method, target), Target::Refuse(_))
    }

    #[test]
    fn inference_paths_classify_with_the_query_verbatim() {
        assert_eq!(
            classify_target("POST", "/v1/messages?beta=true"),
            Target::Inference {
                path: "/v1/messages".to_owned(),
                query: Some("beta=true".to_owned()),
            }
        );
        assert_eq!(
            classify_target("POST", "/v1/messages/count_tokens"),
            Target::Inference {
                path: "/v1/messages/count_tokens".to_owned(),
                query: None,
            }
        );
    }

    #[test]
    fn everything_but_the_two_inference_paths_is_refused() {
        for (method, target) in [
            ("GET", "/v1/messages"),
            ("POST", "/v1/code/sessions"),
            ("GET", "/v1/code/sessions"),
            ("GET", "/v1/code/triggers"),
            ("GET", "/v1/mcp_servers?limit=1"),
            ("GET", "/v1/environments/bridge"),
            ("POST", "/v1/messages/"),
            ("POST", "/v1/messages/batches"),
            ("POST", "/V1/messages"),
            ("POST", "//v1/messages"),
            ("POST", "/v1/messages%2Fcount_tokens"),
            ("POST", "/v1/messages#x"),
            ("POST", "http://api.anthropic.com/v1/messages"),
            ("GET", "http://evil.example/x"),
            ("POST", "*"),
            ("OPTIONS", "*"),
            ("POST", ""),
        ] {
            assert!(refused(method, target), "{method} {target}");
        }
    }

    #[test]
    fn control_and_non_ascii_bytes_in_the_target_are_refused() {
        for target in [
            "/v1/messages?a=\u{7f}",
            "/v1/messages?a=é",
            "/v1/messages?a=\t1",
        ] {
            assert!(refused("POST", target), "{target:?}");
        }
    }

    #[test]
    fn connect_targets_are_authority_form() {
        assert_eq!(
            classify_target("CONNECT", "static.crates.io:443"),
            Target::Connect {
                host: "static.crates.io".to_owned(),
                port: 443,
            }
        );
        assert_eq!(
            classify_target("CONNECT", "[::1]:443"),
            Target::Connect {
                host: "[::1]".to_owned(),
                port: 443,
            }
        );
        for target in [
            "static.crates.io",
            ":443",
            "host:",
            "host:99999",
            "host:44a",
            "user@host:443",
            "host/path:443",
        ] {
            assert!(refused("CONNECT", target), "{target:?}");
        }
    }

    #[test]
    fn the_clients_credentials_and_hop_by_hop_headers_are_dropped() {
        let sent: Vec<Header> = [
            ("Authorization", "Bearer dummy"),
            ("X-Api-Key", "k"),
            ("COOKIE", "c=1"),
            ("Host", "evil"),
            ("Connection", "keep-alive"),
            ("Keep-Alive", "5"),
            ("TE", "trailers"),
            ("Trailer", "x"),
            ("Upgrade", "websocket"),
            ("Transfer-Encoding", "chunked"),
            ("Content-Length", "3"),
            ("Expect", "100-continue"),
            ("Proxy-Authorization", "Basic x"),
            ("Proxy-Connection", "keep-alive"),
            ("anthropic-version", "2023-06-01"),
            ("Anthropic-Beta", "oauth-2025-04-20"),
            ("x-stainless-os", "Linux"),
            ("User-Agent", "claude-cli"),
            ("content-type", "application/json"),
            ("accept", "application/json"),
        ]
        .iter()
        .map(|(n, v)| ((*n).to_owned(), v.as_bytes().to_vec()))
        .collect();
        let kept: Vec<String> = forwarded_headers(&sent)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(
            kept,
            [
                "anthropic-version",
                "Anthropic-Beta",
                "x-stainless-os",
                "User-Agent",
                "content-type",
                "accept"
            ]
        );
    }
}
