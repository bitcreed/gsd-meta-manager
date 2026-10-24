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

/// The server-side tool types an inference body may carry: `web_search_*` only
/// (D-05). Every other server tool (`web_fetch_*`, `code_execution_*`,
/// `advisor_*`, `mcp_toolset`, `computer_*`, `tool_search_*`) asks Anthropic's
/// servers to fetch, run or connect on the driver's behalf.
const ALLOWED_TOOL_TYPE_PREFIX: &str = "web_search_";

/// Block types that are server-side reach by themselves, wherever they nest.
const DENIED_BLOCK_TYPES: &[&str] = &["container_upload", "mcp_tool_use", "mcp_tool_result"];

/// Judge a parsed `/v1/messages` body: forward only what asks the server for
/// nothing beyond inference and `web_search` (D-05, RESEARCH Q1 rules 3-5).
///
/// A tool-type ALLOWLIST rather than a list of known-bad types, because the
/// server-tool family keeps growing; and a recursive walk for `source.type` of
/// `url` or `file`, because those content blocks are the same server-side fetch
/// channel through `messages[]` (an image URL inside a `tool_result` included).
/// The reason names the field or the tool-type string (policy data) and never a
/// URL or a value from the body.
pub fn inference_body_verdict(body: &serde_json::Value) -> BodyVerdict {
    use serde_json::Value;
    let Some(object) = body.as_object() else {
        return BodyVerdict::Deny("body is not a JSON object".to_owned());
    };
    if object.contains_key("mcp_servers") {
        return BodyVerdict::Deny("mcp_servers".to_owned());
    }
    if object.get("container").is_some_and(|c| !c.is_null()) {
        return BodyVerdict::Deny("container".to_owned());
    }
    match object.get("tools") {
        None | Some(Value::Null) => {}
        Some(Value::Array(tools)) => {
            for tool in tools {
                let Value::Object(tool) = tool else {
                    return BodyVerdict::Deny("a tools entry is not an object".to_owned());
                };
                match tool.get("type") {
                    None => {}
                    Some(Value::String(kind))
                        if kind == "custom" || kind.starts_with(ALLOWED_TOOL_TYPE_PREFIX) => {}
                    Some(Value::String(kind)) => {
                        return BodyVerdict::Deny(format!("server tool {}", policy_word(kind)));
                    }
                    Some(_) => {
                        return BodyVerdict::Deny("a tool type is not a string".to_owned());
                    }
                }
            }
        }
        Some(_) => return BodyVerdict::Deny("tools is not a list".to_owned()),
    }
    match remote_source(body) {
        Some(why) => BodyVerdict::Deny(why),
        None => BodyVerdict::Forward,
    }
}

/// Why `value` (or anything inside it) reaches outside the request, or `None`.
fn remote_source(value: &serde_json::Value) -> Option<String> {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(source)) = map.get("source") {
                if let Some(kind @ ("url" | "file")) = source.get("type").and_then(Value::as_str) {
                    return Some(format!("{kind}-sourced content block"));
                }
            }
            if let Some(kind) = map.get("type").and_then(Value::as_str) {
                if DENIED_BLOCK_TYPES.contains(&kind) {
                    return Some(format!("{kind} block"));
                }
            }
            map.values().find_map(remote_source)
        }
        Value::Array(items) => items.iter().find_map(remote_source),
        _ => None,
    }
}

/// A request-derived string reduced to what can safely be echoed in a refusal:
/// the identity alphabet (ASCII letters, digits and `_ . -`, delegated to
/// `text::is_identity_char`), anything else becoming `?`, at most 48 characters.
fn policy_word(text: &str) -> String {
    text.chars()
        .map(|c| {
            if crate::text::is_identity_char(c) {
                c
            } else {
                '?'
            }
        })
        .take(48)
        .collect()
}

/// Parse `bytes` as JSON and judge it: malformed JSON, a body that is not an
/// object and JSON nested past serde_json's recursion limit are all denied, so
/// nothing the proxy could not fully read is forwarded.
pub fn judge_body(bytes: &[u8]) -> BodyVerdict {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(value) => inference_body_verdict(&value),
        Err(_) => BodyVerdict::Deny("body is not valid JSON".to_owned()),
    }
}

/// The header values called `name` as text; a value that is not UTF-8 is `None`.
fn values<'a>(headers: &'a [Header], name: &'a str) -> impl Iterator<Item = Option<&'a str>> {
    headers
        .iter()
        .filter(move |h| header_is(h, name))
        .map(|(_, value)| std::str::from_utf8(value).ok())
}

/// How the request body is delimited, or why the proxy will not judge it
/// (RESEARCH Q1 rule 2, Pitfall 3).
///
/// Refused: an `Upgrade`; a `Content-Encoding` other than `identity` (the body
/// could not be inspected); an `Expect` other than `100-continue`; both
/// `Content-Length` and `Transfer-Encoding`, or either twice (a second request
/// could ride along); a `Transfer-Encoding` other than a lone `chunked`; a
/// `Content-Length` that is not plain decimal digits. A length over
/// [`BODY_MAX_BYTES`](super::http1::BODY_MAX_BYTES) is [`Denial::TooLarge`],
/// decided from the head alone.
pub fn framing_verdict(headers: &[Header]) -> Result<Framing, Denial> {
    let bad = |reason: &str| Err(Denial::BadRequest(reason.to_owned()));
    if headers.iter().any(|h| header_is(h, "upgrade")) {
        return bad("Upgrade is not supported");
    }
    for value in values(headers, "content-encoding") {
        let identity = value.is_some_and(|v| {
            v.split(',')
                .all(|token| token.trim().eq_ignore_ascii_case("identity"))
        });
        if !identity {
            return bad("only an identity Content-Encoding is accepted");
        }
    }
    for value in values(headers, "expect") {
        if !value.is_some_and(|v| v.trim().eq_ignore_ascii_case("100-continue")) {
            return bad("only Expect: 100-continue is supported");
        }
    }
    let lengths: Vec<Option<&str>> = values(headers, "content-length").collect();
    let encodings: Vec<Option<&str>> = values(headers, "transfer-encoding").collect();
    if !lengths.is_empty() && !encodings.is_empty() {
        return bad("both Content-Length and Transfer-Encoding");
    }
    match (lengths.as_slice(), encodings.as_slice()) {
        ([], []) => Ok(Framing::Length(0)),
        ([], [encoding]) => {
            if encoding.is_some_and(|e| e.trim().eq_ignore_ascii_case("chunked")) {
                Ok(Framing::Chunked)
            } else {
                bad("only a chunked Transfer-Encoding is supported")
            }
        }
        ([], _) => bad("more than one Transfer-Encoding header"),
        ([length], []) => {
            let Some(digits) = length
                .map(str::trim)
                .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
            else {
                return bad("Content-Length is not a number");
            };
            let cap = super::http1::BODY_MAX_BYTES as u64;
            match digits.parse::<u64>() {
                Ok(n) if n <= cap => Ok(Framing::Length(n)),
                _ => Err(Denial::TooLarge("request body is too large".to_owned())),
            }
        }
        _ => bad("more than one Content-Length header"),
    }
}

/// Whether the client sent `Expect: 100-continue` and waits for the interim
/// response before sending the body. Only meaningful after [`framing_verdict`]
/// accepted the headers.
pub fn wants_continue(headers: &[Header]) -> bool {
    values(headers, "expect")
        .any(|v| v.is_some_and(|v| v.trim().eq_ignore_ascii_case("100-continue")))
}

/// Cloud metadata singletons: addresses a sandbox must never reach through a
/// name that resolves to them.
const METADATA_V4: &[[u8; 4]] = &[
    [169, 254, 169, 254], // AWS, GCP, Azure, most clouds
    [169, 254, 170, 2],   // AWS ECS task metadata
    [100, 100, 100, 200], // Alibaba
    [168, 63, 129, 16],   // Azure wire server
    [192, 0, 0, 192],     // Oracle
];

/// The class of an address a `CONNECT` must never reach, or `None` for a public
/// one (D-06, RESEARCH Pattern 3).
///
/// The class names the kind of address and never the address. Every IPv6 form
/// that carries an IPv4 address (IPv4-mapped `::ffff:0:0/96`, the deprecated
/// IPv4-compatible `::a.b.c.d`, NAT64 `64:ff9b::/96` and 6to4 `2002::/16`) is
/// judged as that IPv4 address, so a name cannot smuggle `10.0.0.1` past the
/// guard in an IPv6 coat.
pub fn ip_denied(ip: std::net::IpAddr) -> Option<&'static str> {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => v4_class(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return v4_class(v4);
            }
            let s = v6.segments();
            if v6.is_unspecified() {
                return Some("unspecified");
            }
            if v6.is_loopback() {
                return Some("loopback");
            }
            let embedded = |hi: u16, lo: u16| {
                v4_class(std::net::Ipv4Addr::new(
                    (hi >> 8) as u8,
                    hi as u8,
                    (lo >> 8) as u8,
                    lo as u8,
                ))
            };
            if s[..6] == [0, 0, 0, 0, 0, 0] {
                // The deprecated IPv4-compatible form: judged as its IPv4 address,
                // and refused as reserved when that address is public.
                return embedded(s[6], s[7]).or(Some("reserved"));
            }
            if s[0] == 0x0064 && s[1] == 0xff9b && s[2..6] == [0, 0, 0, 0] {
                return embedded(s[6], s[7]);
            }
            if s[0] == 0x2002 {
                return embedded(s[1], s[2]);
            }
            if s[0] == 0xfd00 && s[1] == 0x0ec2 {
                return Some("metadata");
            }
            if v6.is_multicast() {
                return Some("multicast");
            }
            if (s[0] & 0xfe00) == 0xfc00 {
                return Some("ula");
            }
            if (s[0] & 0xffc0) == 0xfe80 {
                return Some("link-local");
            }
            None
        }
    }
}

fn v4_class(v4: std::net::Ipv4Addr) -> Option<&'static str> {
    let o = v4.octets();
    if METADATA_V4.contains(&o) {
        Some("metadata")
    } else if v4.is_loopback() {
        Some("loopback")
    } else if v4.is_unspecified() || o[0] == 0 {
        Some("unspecified")
    } else if v4.is_private() {
        Some("private")
    } else if v4.is_link_local() {
        Some("link-local")
    } else if o[0] == 100 && (o[1] & 0xc0) == 64 {
        Some("cgnat")
    } else if v4.is_broadcast() {
        Some("broadcast")
    } else if v4.is_multicast() {
        Some("multicast")
    } else if o[0] >= 240 || (o[0] == 192 && o[1] == 0 && o[2] == 0) {
        Some("reserved")
    } else {
        None
    }
}

/// A host name as compared: ASCII-lowercase, one trailing root dot removed.
fn normalize_host(host: &str) -> String {
    host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase()
}

/// Whether `host` is, or would be read by a resolver as, an IP literal: bracketed,
/// a parseable address, or ending in a numeric label (`2130706433`, `127.1`,
/// `0x7f.0.0.1`, `010.0.0.1`), which `getaddrinfo` takes as an IPv4 address.
fn looks_like_ip_literal(host: &str) -> bool {
    if host.starts_with('[') || host.contains(':') || host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    let last = host.rsplit('.').next().unwrap_or(host);
    let numeric = !last.is_empty() && last.bytes().all(|b| b.is_ascii_digit());
    let hex = last
        .strip_prefix("0x")
        .or_else(|| last.strip_prefix("0X"))
        .is_some_and(|digits| digits.bytes().all(|b| b.is_ascii_hexdigit()));
    numeric || hex
}

/// May a `CONNECT` to `host:port` be tunnelled, given the run's allowlist (D-06)?
///
/// Deny by default. Refused: any IP literal, even an allowlisted one (the
/// allowlist is names only); `api.anthropic.com`, even allowlisted (inference
/// goes only through the reverse proxy, which holds the token and judges the
/// body); any port but 443; any name not in `allow_hosts` (exact,
/// ASCII-case-insensitive, root dot ignored). Every refusal is a
/// [`Denial::Forbidden`] whose reason is policy text.
pub fn connect_verdict(host: &str, port: u16, allow_hosts: &[String]) -> Result<(), Denial> {
    let no = |reason: &str| Err(Denial::Forbidden(reason.to_owned()));
    if looks_like_ip_literal(host) {
        return no("CONNECT to an IP literal is refused");
    }
    let host = normalize_host(host);
    if host == super::INFERENCE_HOST {
        return no("the inference host is reachable only through the inference proxy");
    }
    if port != 443 {
        return no("CONNECT is limited to port 443");
    }
    if allow_hosts
        .iter()
        .any(|allowed| normalize_host(allowed) == host)
    {
        Ok(())
    } else {
        no("host is not on the egress allowlist")
    }
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

    // ----------------------------------------------------------- ip_denied

    fn class(ip: &str) -> Option<&'static str> {
        ip_denied(ip.parse().expect("an IP literal"))
    }

    #[test]
    fn denied_addresses_name_their_class() {
        for (ip, expected) in [
            ("127.0.0.1", "loopback"),
            ("127.1.2.3", "loopback"),
            ("0.0.0.0", "unspecified"),
            ("0.1.2.3", "unspecified"),
            ("10.1.2.3", "private"),
            ("172.16.0.1", "private"),
            ("172.31.255.255", "private"),
            ("192.168.1.1", "private"),
            ("169.254.169.254", "metadata"),
            ("169.254.1.1", "link-local"),
            ("100.64.0.1", "cgnat"),
            ("100.127.255.255", "cgnat"),
            ("100.100.100.200", "metadata"),
            ("168.63.129.16", "metadata"),
            ("192.0.0.192", "metadata"),
            ("224.0.0.1", "multicast"),
            ("239.255.255.255", "multicast"),
            ("255.255.255.255", "broadcast"),
            ("240.0.0.1", "reserved"),
            ("::1", "loopback"),
            ("::", "unspecified"),
            ("fe80::1", "link-local"),
            ("fc00::1", "ula"),
            ("fd12:3456::1", "ula"),
            ("fd00:ec2::254", "metadata"),
            ("ff02::1", "multicast"),
            // IPv6 forms that carry an IPv4 address are judged as that address.
            ("::ffff:10.0.0.1", "private"),
            ("::ffff:127.0.0.1", "loopback"),
            ("::ffff:169.254.169.254", "metadata"),
            ("64:ff9b::a00:1", "private"),
            ("64:ff9b::7f00:1", "loopback"),
            ("2002:a00:1::", "private"),
            ("2002:7f00:1::1", "loopback"),
            ("::10.0.0.1", "private"),
        ] {
            assert_eq!(class(ip), Some(expected), "{ip}");
        }
    }

    #[test]
    fn public_addresses_are_not_denied() {
        for ip in [
            "1.1.1.1",
            "8.8.8.8",
            "140.82.112.3",
            "100.63.255.255",
            "100.128.0.1",
            "172.15.255.255",
            "172.32.0.1",
            "::ffff:1.1.1.1",
            "64:ff9b::101:101",
            "2002:101:101::",
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
        ] {
            assert_eq!(class(ip), None, "{ip}");
        }
    }

    // ------------------------------------------------------ connect_verdict

    fn allow(hosts: &[&str]) -> Vec<String> {
        hosts.iter().map(|h| (*h).to_owned()).collect()
    }

    fn connect_reason(host: &str, port: u16, hosts: &[&str]) -> Option<String> {
        match connect_verdict(host, port, &allow(hosts)) {
            Ok(()) => None,
            Err(Denial::Forbidden(reason)) => Some(reason),
            Err(other) => panic!("a CONNECT refusal is a 403, got {other:?}"),
        }
    }

    #[test]
    fn connect_is_deny_by_default_and_allows_exact_names_on_443() {
        assert!(connect_reason("example.org", 443, &[])
            .expect("denied")
            .contains("not on the egress allowlist"));
        assert_eq!(
            connect_reason("static.crates.io", 443, &["static.crates.io"]),
            None
        );
        assert_eq!(
            connect_reason("STATIC.Crates.IO", 443, &["static.crates.io"]),
            None
        );
        assert_eq!(
            connect_reason("static.crates.io", 443, &["Static.Crates.Io"]),
            None
        );
        assert_eq!(
            connect_reason("static.crates.io.", 443, &["static.crates.io"]),
            None
        );
        // Exact membership: a suffix, a prefix or a parent domain is not a match.
        for host in [
            "crates.io",
            "evil-static.crates.io",
            "static.crates.io.evil.example",
        ] {
            assert!(
                connect_reason(host, 443, &["static.crates.io"]).is_some(),
                "{host}"
            );
        }
    }

    #[test]
    fn connect_never_tunnels_the_inference_host() {
        for host in [
            "api.anthropic.com",
            "API.Anthropic.COM",
            "api.anthropic.com.",
        ] {
            let reason = connect_reason(host, 443, &["api.anthropic.com"]).expect("denied");
            assert!(reason.contains("inference"), "{host}: {reason}");
        }
    }

    #[test]
    fn connect_is_limited_to_port_443() {
        for port in [0, 22, 80, 8443, 65535] {
            let reason =
                connect_reason("static.crates.io", port, &["static.crates.io"]).expect("denied");
            assert!(reason.contains("443"), "{port}: {reason}");
        }
    }

    #[test]
    fn connect_refuses_ip_literals_even_when_allowlisted() {
        for host in [
            "1.2.3.4",
            "127.0.0.1",
            "[::1]",
            "[2606:4700:4700::1111]",
            "::1",
            "2130706433",
            "0x7f.0.0.1",
            "127.1",
            "010.0.0.1",
        ] {
            let reason = connect_reason(host, 443, &[host]).expect("denied");
            assert!(reason.contains("IP literal"), "{host}: {reason}");
        }
    }
}
