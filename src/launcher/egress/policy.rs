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

/// The verdict on an inference request body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyVerdict {
    Forward,
    /// Refused; the text names the field or the tool-type string (policy data,
    /// never a URL or a value from the body).
    Deny(String),
}

/// Why a request is refused, and with which status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denial {
    /// 403: the policy forbids it.
    Forbidden(String),
    /// 400: the request is malformed or framed in a way the proxy will not judge.
    BadRequest(String),
    /// 413: over the body bound.
    TooLarge(String),
}

impl Denial {
    pub fn status(&self) -> u16 {
        match self {
            Denial::Forbidden(_) => 403,
            Denial::BadRequest(_) => 400,
            Denial::TooLarge(_) => 413,
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            Denial::Forbidden(r) | Denial::BadRequest(r) | Denial::TooLarge(r) => r,
        }
    }
}

/// How the request body is delimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    Length(u64),
    Chunked,
}

/// RED stub: judged in the GREEN commit.
pub fn inference_body_verdict(_body: &serde_json::Value) -> BodyVerdict {
    BodyVerdict::Forward
}

/// RED stub: judged in the GREEN commit.
pub fn judge_body(_bytes: &[u8]) -> BodyVerdict {
    BodyVerdict::Forward
}

/// RED stub: judged in the GREEN commit.
pub fn framing_verdict(_headers: &[Header]) -> Result<Framing, Denial> {
    Ok(Framing::Length(0))
}

/// RED stub: judged in the GREEN commit.
pub fn wants_continue(_headers: &[Header]) -> bool {
    false
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

    // ---------------------------------------------------------- body verdict

    use serde_json::json;

    fn denied(body: serde_json::Value) -> bool {
        matches!(inference_body_verdict(&body), BodyVerdict::Deny(_))
    }

    #[test]
    fn client_tools_and_web_search_are_forwarded() {
        for body in [
            json!({"model": "m", "messages": [], "tools": [{"name": "Bash", "input_schema": {}}]}),
            json!({"model": "m", "messages": [], "tools": [
                {"name": "Bash", "input_schema": {}},
                {"type": "web_search_20250305", "name": "web_search", "max_uses": 5}]}),
            json!({"model": "m", "messages": [], "tools": [
                {"type": "custom", "name": "x", "input_schema": {}},
                {"type": "web_search_20260209", "name": "web_search"}]}),
            json!({"model": "m", "messages": [], "container": null}),
            json!({"model": "m", "messages": [{"role": "user", "content": [
                {"type": "text", "text": "hi"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "AAAA"}},
                {"type": "search_result", "source": "https://x.example/a", "title": "t",
                 "content": [{"type": "text", "text": "r"}]}]}]}),
        ] {
            assert_eq!(
                inference_body_verdict(&body),
                BodyVerdict::Forward,
                "{body}"
            );
        }
    }

    #[test]
    fn server_side_reach_is_denied_at_the_top_level_and_in_tools() {
        for body in [
            json!({"messages": [], "mcp_servers": []}),
            json!({"messages": [], "mcp_servers": [{"type": "url", "url": "https://x.example"}]}),
            json!({"messages": [], "container": {"skills": []}}),
            json!({"messages": [], "container": "container_011"}),
            json!({"messages": [], "tools": [{"type": "web_fetch_20260318", "name": "web_fetch"}]}),
            json!({"messages": [], "tools": [{"type": "web_fetch_20250910", "name": "web_fetch"}]}),
            json!({"messages": [], "tools": [{"type": "code_execution_20260521", "name": "code_execution"}]}),
            json!({"messages": [], "tools": [{"type": "advisor_20260301", "name": "advisor"}]}),
            json!({"messages": [], "tools": [{"type": "mcp_toolset", "mcp_server_name": "x"}]}),
            json!({"messages": [], "tools": [{"type": "computer_20251124", "name": "computer"}]}),
            json!({"messages": [], "tools": [{"type": "bash_20250124", "name": "bash"}]}),
            json!({"messages": [], "tools": [{"name": "Bash"}, {"type": "web_fetch_20260318"}]}),
            json!({"messages": [], "tools": [{"type": 5, "name": "x"}]}),
            json!({"messages": [], "tools": ["web_fetch"]}),
            json!({"messages": [], "tools": {"type": "web_fetch_20260318"}}),
        ] {
            assert!(denied(body.clone()), "{body}");
        }
    }

    #[test]
    fn remote_sources_and_server_blocks_are_denied_wherever_they_nest() {
        let url_image =
            json!({"type": "image", "source": {"type": "url", "url": "https://x.example/?d=1"}});
        for body in [
            json!({"messages": [{"role": "user", "content": [url_image.clone()]}]}),
            json!({"messages": [{"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "t", "content": [url_image.clone()]}]}]}),
            json!({"messages": [{"role": "user", "content": [
                {"type": "document", "source": {"type": "file", "file_id": "file_01"}}]}]}),
            json!({"system": [{"type": "document", "source": {"type": "url", "url": "https://x.example/a.pdf"}}],
                   "messages": []}),
            json!({"messages": [{"role": "user", "content": [
                {"type": "container_upload", "file_id": "file_01"}]}]}),
            json!({"messages": [{"role": "assistant", "content": [
                {"type": "mcp_tool_use", "id": "m", "name": "n", "server_name": "s", "input": {}}]}]}),
            json!({"messages": [{"role": "user", "content": [
                {"type": "tool_result", "content": [
                    {"type": "mcp_tool_result", "tool_use_id": "m", "content": []}]}]}]}),
        ] {
            assert!(denied(body.clone()), "{body}");
        }
    }

    #[test]
    fn a_denial_names_the_field_or_type_and_never_echoes_a_url() {
        let url = "https://attacker.example/?d=SECRET";
        let BodyVerdict::Deny(reason) =
            inference_body_verdict(&json!({"messages": [{"role": "user",
            "content": [{"type": "image", "source": {"type": "url", "url": url}}]}]}))
        else {
            panic!("a url-sourced image must be denied");
        };
        assert!(
            !reason.contains("attacker") && !reason.contains("SECRET"),
            "{reason}"
        );
        let BodyVerdict::Deny(reason) = inference_body_verdict(
            &json!({"messages": [], "tools": [{"type": "web_fetch_20260318"}]}),
        ) else {
            panic!("a web_fetch tool must be denied");
        };
        assert!(reason.contains("web_fetch_20260318"), "{reason}");
    }

    #[test]
    fn a_body_that_is_not_an_object_or_not_json_is_denied() {
        for bytes in [
            &b"[]"[..],
            b"\"text\"",
            b"null",
            b"42",
            b"",
            b"{",
            b"{\"model\": }",
            b"\xff\xfe",
        ] {
            assert!(
                matches!(judge_body(bytes), BodyVerdict::Deny(_)),
                "{:?}",
                String::from_utf8_lossy(bytes)
            );
        }
        // Nested past serde_json's recursion limit (128): a parse error, a deny.
        let deep = format!("{}1{}", "[".repeat(300), "]".repeat(300));
        assert!(matches!(judge_body(deep.as_bytes()), BodyVerdict::Deny(_)));
        let deep_object = format!("{{\"a\":{}1{}}}", "[".repeat(300), "]".repeat(300));
        assert!(matches!(
            judge_body(deep_object.as_bytes()),
            BodyVerdict::Deny(_)
        ));
        assert_eq!(
            judge_body(br#"{"model":"m","messages":[]}"#),
            BodyVerdict::Forward
        );
    }

    // ------------------------------------------------------------- framing

    fn hdrs(pairs: &[(&str, &str)]) -> Vec<Header> {
        pairs
            .iter()
            .map(|(n, v)| ((*n).to_owned(), v.as_bytes().to_vec()))
            .collect()
    }

    #[test]
    fn framing_accepts_one_length_or_chunked() {
        assert_eq!(
            framing_verdict(&hdrs(&[("Content-Length", "12")])),
            Ok(Framing::Length(12))
        );
        assert_eq!(
            framing_verdict(&hdrs(&[("content-length", "33554432")])),
            Ok(Framing::Length(33_554_432))
        );
        assert_eq!(
            framing_verdict(&hdrs(&[("Transfer-Encoding", "chunked")])),
            Ok(Framing::Chunked)
        );
        assert_eq!(
            framing_verdict(&hdrs(&[("Transfer-Encoding", " Chunked ")])),
            Ok(Framing::Chunked)
        );
        assert_eq!(framing_verdict(&hdrs(&[])), Ok(Framing::Length(0)));
        assert_eq!(
            framing_verdict(&hdrs(&[
                ("Content-Length", "2"),
                ("Content-Encoding", "identity")
            ])),
            Ok(Framing::Length(2))
        );
    }

    #[test]
    fn framing_refuses_ambiguity_and_encodings_it_cannot_inspect() {
        for headers in [
            hdrs(&[("Content-Length", "2"), ("Transfer-Encoding", "chunked")]),
            hdrs(&[("Content-Length", "2"), ("Content-Length", "2")]),
            hdrs(&[("Content-Length", "2, 2")]),
            hdrs(&[("Content-Length", "+2")]),
            hdrs(&[("Content-Length", "-1")]),
            hdrs(&[("Content-Length", "")]),
            hdrs(&[("Content-Length", "0x10")]),
            hdrs(&[("Transfer-Encoding", "gzip, chunked")]),
            hdrs(&[("Transfer-Encoding", "identity")]),
            hdrs(&[
                ("Transfer-Encoding", "chunked"),
                ("Transfer-Encoding", "chunked"),
            ]),
            hdrs(&[("Content-Length", "2"), ("Content-Encoding", "gzip")]),
            hdrs(&[("Content-Length", "2"), ("Content-Encoding", "br")]),
            hdrs(&[
                ("Content-Length", "2"),
                ("Content-Encoding", "identity, gzip"),
            ]),
            hdrs(&[("Content-Length", "2"), ("Upgrade", "websocket")]),
            hdrs(&[("Content-Length", "2"), ("Expect", "something-else")]),
        ] {
            assert!(
                matches!(framing_verdict(&headers), Err(Denial::BadRequest(_))),
                "{headers:?}"
            );
        }
    }

    #[test]
    fn framing_refuses_a_body_over_the_cap_before_reading_it() {
        for length in ["33554433", "99999999999", "18446744073709551616"] {
            assert!(
                matches!(
                    framing_verdict(&hdrs(&[("Content-Length", length)])),
                    Err(Denial::TooLarge(_))
                ),
                "{length}"
            );
        }
    }

    #[test]
    fn expect_100_continue_is_accepted_and_flagged() {
        let headers = hdrs(&[("Content-Length", "2"), ("Expect", "100-continue")]);
        assert_eq!(framing_verdict(&headers), Ok(Framing::Length(2)));
        assert!(wants_continue(&headers));
        assert!(!wants_continue(&hdrs(&[("Content-Length", "2")])));
    }
}
