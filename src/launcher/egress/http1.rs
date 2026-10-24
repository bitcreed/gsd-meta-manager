//! Bounded HTTP/1 reading and canonical request writing (28.1-01).
//!
//! The driver on the other end of the socket is untrusted (T-28.1-05): every read
//! here has a byte cap AND a total deadline, so neither a slow nor an endless
//! sender can pin the server. Nothing the driver sent is forwarded as it arrived:
//! the head is parsed with `httparse`, the body is buffered whole, and the
//! upstream request is written from scratch by [`request_bytes`] with its own
//! `Content-Length` and `Connection: close`.

use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};

use super::policy::{Framing, Header};
use super::INFERENCE_HOST;

/// The most a request head (request line, headers and the blank line) may take.
pub const HEAD_MAX_BYTES: usize = 65536;

/// The most a request body may take: the API's own 32 MB standard-endpoint bound.
pub const BODY_MAX_BYTES: usize = 33_554_432;

/// The TOTAL time a client has to deliver a complete head.
pub const HEAD_TIMEOUT: Duration = Duration::from_secs(30);

/// The TOTAL time a client has to deliver a complete body once the head is in.
pub const BODY_TIMEOUT: Duration = Duration::from_secs(120);

/// `httparse` header slots; a head with more headers is [`ReadError::HeadTooLarge`].
const MAX_HEADERS: usize = 64;

/// A parsed request head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub method: String,
    pub target: String,
    /// The `x` of `HTTP/1.x`.
    pub minor_version: u8,
    pub headers: Vec<Header>,
}

/// Why a read produced no complete head or body.
#[derive(Debug)]
pub enum ReadError {
    /// The deadline passed before the head or body was complete.
    Timeout,
    /// The head is over [`HEAD_MAX_BYTES`] or has more than 64 headers.
    HeadTooLarge,
    /// The body is over [`BODY_MAX_BYTES`].
    BodyTooLarge,
    /// The bytes are not an HTTP/1 request.
    Malformed(String),
    /// The client closed the connection before the message was complete.
    Closed,
    /// A read fault.
    Io(std::io::Error),
}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        ReadError::Io(e)
    }
}

/// Read one request head, within [`HEAD_TIMEOUT`] and [`HEAD_MAX_BYTES`].
///
/// Returns the head and every byte already read past it (the start of the body,
/// or the start of a pipelined request that is never served: one request per
/// connection).
pub async fn read_head<R>(reader: &mut R) -> Result<(Head, Vec<u8>), ReadError>
where
    R: AsyncRead + Unpin,
{
    match tokio::time::timeout(HEAD_TIMEOUT, read_head_inner(reader)).await {
        Ok(result) => result,
        Err(_) => Err(ReadError::Timeout),
    }
}

async fn read_head_inner<R>(reader: &mut R) -> Result<(Head, Vec<u8>), ReadError>
where
    R: AsyncRead + Unpin,
{
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut scanned = 0usize;
    loop {
        if let Some(end) = find_head_end(&buf, scanned) {
            let head = parse_head(&buf[..end])?;
            return Ok((head, buf[end..].to_vec()));
        }
        // A terminator can straddle two reads: rescan the last three bytes.
        scanned = buf.len().saturating_sub(3);
        if buf.len() >= HEAD_MAX_BYTES {
            return Err(ReadError::HeadTooLarge);
        }
        let mut chunk = [0u8; 4096];
        let room = (HEAD_MAX_BYTES - buf.len()).min(chunk.len());
        let n = reader.read(&mut chunk[..room]).await?;
        if n == 0 {
            return Err(ReadError::Closed);
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// The index just past the first `\r\n\r\n` at or after `from`.
fn find_head_end(buf: &[u8], from: usize) -> Option<usize> {
    buf.get(from..)?
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| from + i + 4)
}

fn parse_head(bytes: &[u8]) -> Result<Head, ReadError> {
    let mut slots = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut request = httparse::Request::new(&mut slots);
    match request.parse(bytes) {
        Ok(httparse::Status::Complete(n)) if n == bytes.len() => {}
        Ok(_) => return Err(ReadError::Malformed("incomplete request head".to_owned())),
        Err(httparse::Error::TooManyHeaders) => return Err(ReadError::HeadTooLarge),
        Err(e) => return Err(ReadError::Malformed(e.to_string())),
    }
    let (Some(method), Some(target), Some(minor_version)) =
        (request.method, request.path, request.version)
    else {
        return Err(ReadError::Malformed("incomplete request line".to_owned()));
    };
    Ok(Head {
        method: method.to_owned(),
        target: target.to_owned(),
        minor_version,
        headers: request
            .headers
            .iter()
            .map(|h| (h.name.to_owned(), h.value.to_vec()))
            .collect(),
    })
}

/// Read the request body the head announced, `leftover` first, within
/// [`BODY_TIMEOUT`] and [`BODY_MAX_BYTES`]. A chunked body is decoded.
pub async fn read_body<R>(
    reader: &mut R,
    leftover: Vec<u8>,
    framing: Framing,
) -> Result<Vec<u8>, ReadError>
where
    R: AsyncRead + Unpin,
{
    match framing {
        Framing::Length(len) => {
            let len = usize::try_from(len).map_err(|_| ReadError::BodyTooLarge)?;
            read_length_body(reader, leftover, len).await
        }
        Framing::Chunked => read_chunked_body(reader, leftover).await,
    }
}

/// A `Content-Length` body. The buffer grows with what actually arrives, never to
/// the claimed length up front, so a lying `Content-Length` costs nothing.
async fn read_length_body<R>(
    reader: &mut R,
    leftover: Vec<u8>,
    len: usize,
) -> Result<Vec<u8>, ReadError>
where
    R: AsyncRead + Unpin,
{
    if len > BODY_MAX_BYTES {
        return Err(ReadError::BodyTooLarge);
    }
    let fill = async {
        let mut body = leftover;
        body.truncate(len);
        let remaining = len - body.len();
        if remaining > 0 {
            let got = (&mut *reader)
                .take(remaining as u64)
                .read_to_end(&mut body)
                .await?;
            if got < remaining {
                return Err(ReadError::Closed);
            }
        }
        Ok(body)
    };
    match tokio::time::timeout(BODY_TIMEOUT, fill).await {
        Ok(result) => result,
        Err(_) => Err(ReadError::Timeout),
    }
}

/// The longest chunk-size line (size and extensions) or trailer line accepted.
const LINE_MAX_BYTES: usize = 4096;

/// The most trailer lines accepted (and discarded) after the last chunk.
const TRAILER_LINES_MAX: usize = 32;

/// Buffered access to the wire for the chunked decoder, `leftover` first.
struct Wire<'a, R> {
    reader: &'a mut R,
    buf: Vec<u8>,
    pos: usize,
}

impl<R: AsyncRead + Unpin> Wire<'_, R> {
    /// Read more bytes from the wire.
    async fn fill(&mut self) -> Result<(), ReadError> {
        self.buf.drain(..self.pos);
        self.pos = 0;
        let mut chunk = [0u8; 8192];
        let n = self.reader.read(&mut chunk).await?;
        if n == 0 {
            return Err(ReadError::Closed);
        }
        self.buf.extend_from_slice(&chunk[..n]);
        Ok(())
    }

    /// One CRLF-terminated line, without the CRLF. A bare LF is not a line end.
    async fn line(&mut self) -> Result<Vec<u8>, ReadError> {
        loop {
            if let Some(i) = self.buf[self.pos..].windows(2).position(|w| w == b"\r\n") {
                let line = self.buf[self.pos..self.pos + i].to_vec();
                self.pos += i + 2;
                return Ok(line);
            }
            if self.buf.len() - self.pos > LINE_MAX_BYTES {
                return Err(ReadError::Malformed("chunk line is too long".to_owned()));
            }
            self.fill().await?;
        }
    }

    /// Append exactly `n` bytes to `out`.
    async fn take(&mut self, n: usize, out: &mut Vec<u8>) -> Result<(), ReadError> {
        let mut need = n;
        while need > 0 {
            if self.pos == self.buf.len() {
                self.fill().await?;
            }
            let k = (self.buf.len() - self.pos).min(need);
            out.extend_from_slice(&self.buf[self.pos..self.pos + k]);
            self.pos += k;
            need -= k;
        }
        Ok(())
    }
}

/// The size on a chunk-size line: hex digits only, optionally followed by
/// `;extensions`, which are ignored.
fn chunk_size(line: &[u8]) -> Result<u64, ReadError> {
    let malformed = || ReadError::Malformed("chunk size is not hexadecimal".to_owned());
    let digits = line.split(|b| *b == b';').next().unwrap_or(&[]);
    let digits = digits.trim_ascii_end();
    if digits.is_empty() || digits.len() > 16 || !digits.iter().all(u8::is_ascii_hexdigit) {
        return Err(malformed());
    }
    let text = std::str::from_utf8(digits).map_err(|_| malformed())?;
    u64::from_str_radix(text, 16).map_err(|_| malformed())
}

/// Decode a `Transfer-Encoding: chunked` body: hex sizes, extensions ignored,
/// trailers read and discarded, the decoded size capped at [`BODY_MAX_BYTES`]
/// (a chunk that would pass the cap is refused before its data is read).
pub async fn read_chunked_body<R>(reader: &mut R, leftover: Vec<u8>) -> Result<Vec<u8>, ReadError>
where
    R: AsyncRead + Unpin,
{
    let decode = async {
        let mut wire = Wire {
            reader,
            buf: leftover,
            pos: 0,
        };
        let mut body = Vec::new();
        loop {
            let size = chunk_size(&wire.line().await?)?;
            if size == 0 {
                break;
            }
            let room = (BODY_MAX_BYTES - body.len()) as u64;
            if size > room {
                return Err(ReadError::BodyTooLarge);
            }
            wire.take(size as usize, &mut body).await?;
            let mut crlf = Vec::with_capacity(2);
            wire.take(2, &mut crlf).await?;
            if crlf != b"\r\n" {
                return Err(ReadError::Malformed(
                    "chunk data is not followed by CRLF".to_owned(),
                ));
            }
        }
        for _ in 0..TRAILER_LINES_MAX {
            if wire.line().await?.is_empty() {
                return Ok(body);
            }
        }
        Err(ReadError::Malformed("too many trailer lines".to_owned()))
    };
    match tokio::time::timeout(BODY_TIMEOUT, decode).await {
        Ok(result) => result,
        Err(_) => Err(ReadError::Timeout),
    }
}

/// Whether `token` can stand in an `Authorization` header value: non-empty,
/// visible ASCII only. A credentials file with a CR, LF or space in the token
/// must not be able to inject a header into the upstream request.
pub fn token_is_header_safe(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|b| (0x21..=0x7e).contains(&b))
}

/// The canonical upstream request: the kept `headers`, then the four headers the
/// proxy owns (`Host`, `Authorization: Bearer`, `Content-Length`,
/// `Connection: close`), then `body`. `None` when `token` is not header-safe.
///
/// `path_and_query` must already have passed `classify_target`, which admits
/// visible ASCII only; `headers` must already have passed `forwarded_headers`.
pub fn request_bytes(
    method: &str,
    path_and_query: &str,
    headers: &[Header],
    token: &str,
    body: &[u8],
) -> Option<Vec<u8>> {
    if !token_is_header_safe(token) {
        return None;
    }
    let mut out = Vec::with_capacity(body.len() + 1024);
    out.extend_from_slice(format!("{method} {path_and_query} HTTP/1.1\r\n").as_bytes());
    for (name, value) in headers {
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("Host: {INFERENCE_HOST}\r\n").as_bytes());
    out.extend_from_slice(format!("Authorization: Bearer {token}\r\n").as_bytes());
    out.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    out.extend_from_slice(b"Connection: close\r\n\r\n");
    out.extend_from_slice(body);
    Some(out)
}

/// An Anthropic-shaped error response: the status line, `content-type`,
/// `content-length`, `connection: close` and the JSON body
/// `{"type":"error","error":{"type":…,"message":"gsd-meta-manager egress: …"}}`.
///
/// `message` is policy text (a reason, a class name, a tool-type string); callers
/// never pass a header value, a token, a URL from a body or an address.
pub fn error_response(status: u16, message: &str) -> Vec<u8> {
    let (phrase, kind) = match status {
        400 => ("Bad Request", "invalid_request_error"),
        403 => ("Forbidden", "permission_error"),
        408 => ("Request Timeout", "invalid_request_error"),
        413 => ("Payload Too Large", "request_too_large"),
        431 => ("Request Header Fields Too Large", "invalid_request_error"),
        502 => ("Bad Gateway", "api_error"),
        503 => ("Service Unavailable", "overloaded_error"),
        _ => ("Error", "api_error"),
    };
    let body = serde_json::json!({
        "type": "error",
        "error": {
            "type": kind,
            "message": format!("gsd-meta-manager egress: {message}"),
        },
    })
    .to_string();
    format!(
        "HTTP/1.1 {status} {phrase}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_head_is_parsed_and_the_bytes_past_it_are_returned() {
        let wire =
            b"POST /v1/messages?beta=true HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\n\r\n{}tail";
        let mut reader = &wire[..];
        let (head, leftover) = read_head(&mut reader).await.expect("a head");
        assert_eq!(head.method, "POST");
        assert_eq!(head.target, "/v1/messages?beta=true");
        assert_eq!(head.minor_version, 1);
        assert_eq!(head.headers.len(), 2);
        assert_eq!(leftover, b"{}tail");
    }

    #[tokio::test]
    async fn an_oversized_head_is_refused_without_reading_on() {
        let mut wire = b"POST /v1/messages HTTP/1.1\r\nX: ".to_vec();
        wire.resize(HEAD_MAX_BYTES + 10, b'a');
        let mut reader = &wire[..];
        assert!(matches!(
            read_head(&mut reader).await,
            Err(ReadError::HeadTooLarge)
        ));
    }

    #[tokio::test]
    async fn too_many_headers_and_garbage_are_refused() {
        let mut many = b"POST /v1/messages HTTP/1.1\r\n".to_vec();
        for i in 0..70 {
            many.extend_from_slice(format!("h{i}: v\r\n").as_bytes());
        }
        many.extend_from_slice(b"\r\n");
        let mut reader = &many[..];
        assert!(matches!(
            read_head(&mut reader).await,
            Err(ReadError::HeadTooLarge)
        ));
        let mut reader = &b"\x16\x03\x01garbage\r\n\r\n"[..];
        assert!(matches!(
            read_head(&mut reader).await,
            Err(ReadError::Malformed(_))
        ));
    }

    #[tokio::test]
    async fn a_closed_connection_mid_head_is_closed() {
        let mut reader = &b"POST /v1/mess"[..];
        assert!(matches!(
            read_head(&mut reader).await,
            Err(ReadError::Closed)
        ));
    }

    #[tokio::test]
    async fn a_content_length_body_is_read_from_the_leftover_then_the_wire() {
        let mut reader = &b"cdef"[..];
        let body = read_body(&mut reader, b"ab".to_vec(), Framing::Length(6))
            .await
            .expect("body");
        assert_eq!(body, b"abcdef");
        let mut reader = &b""[..];
        let body = read_body(&mut reader, b"abcdefgh".to_vec(), Framing::Length(3))
            .await
            .expect("body");
        assert_eq!(body, b"abc");
    }

    #[tokio::test]
    async fn a_short_body_and_an_oversized_length_are_refused() {
        let mut reader = &b"cd"[..];
        assert!(matches!(
            read_body(&mut reader, b"ab".to_vec(), Framing::Length(6)).await,
            Err(ReadError::Closed)
        ));
        let mut reader = &b""[..];
        assert!(matches!(
            read_body(
                &mut reader,
                Vec::new(),
                Framing::Length(BODY_MAX_BYTES as u64 + 1)
            )
            .await,
            Err(ReadError::BodyTooLarge)
        ));
    }

    #[test]
    fn the_canonical_request_owns_host_authorization_length_and_connection() {
        let headers = vec![("anthropic-version".to_owned(), b"2023-06-01".to_vec())];
        let bytes = request_bytes("POST", "/v1/messages?beta=true", &headers, "T0K", b"{}")
            .expect("a safe token");
        assert_eq!(
            String::from_utf8(bytes).expect("utf8"),
            "POST /v1/messages?beta=true HTTP/1.1\r\n\
             anthropic-version: 2023-06-01\r\n\
             Host: api.anthropic.com\r\n\
             Authorization: Bearer T0K\r\n\
             Content-Length: 2\r\n\
             Connection: close\r\n\r\n{}"
        );
    }

    #[test]
    fn a_token_that_could_inject_a_header_is_refused() {
        for token in ["", "a b", "a\r\nX: y", "tok\u{7f}", "tök"] {
            assert!(request_bytes("POST", "/v1/messages", &[], token, b"").is_none());
        }
    }

    #[test]
    fn an_error_response_is_anthropic_shaped_and_self_framed() {
        let wire = String::from_utf8(error_response(403, "no \"quotes\" survive")).expect("utf8");
        let (head, body) = wire.split_once("\r\n\r\n").expect("a head");
        assert!(head.starts_with("HTTP/1.1 403 Forbidden\r\n"));
        assert!(head.contains("content-type: application/json"));
        assert!(head.contains("connection: close"));
        assert!(head.contains(&format!("content-length: {}", body.len())));
        let json: serde_json::Value = serde_json::from_str(body).expect("json");
        assert_eq!(json["type"], "error");
        assert_eq!(json["error"]["type"], "permission_error");
        assert_eq!(
            json["error"]["message"],
            "gsd-meta-manager egress: no \"quotes\" survive"
        );
    }

    #[tokio::test]
    async fn a_chunked_body_is_decoded_extensions_and_trailers_discarded() {
        let wire = b"5\r\nhello\r\n6;ext=1;other=\"x\"\r\n world\r\n0\r\nTrailer-One: a\r\nTrailer-Two: b\r\n\r\n";
        let mut reader = &wire[..];
        let body = read_chunked_body(&mut reader, Vec::new())
            .await
            .expect("body");
        assert_eq!(body, b"hello world");
    }

    #[tokio::test]
    async fn a_chunked_body_may_start_in_the_leftover_and_continue_on_the_wire() {
        let mut reader = &b"llo\r\n0\r\n\r\n"[..];
        let body = read_chunked_body(&mut reader, b"5\r\nhe".to_vec())
            .await
            .expect("body");
        assert_eq!(body, b"hello");
    }

    #[tokio::test]
    async fn malformed_chunking_is_refused() {
        for wire in [
            &b"zz\r\nhello\r\n0\r\n\r\n"[..],
            b"+5\r\nhello\r\n0\r\n\r\n",
            b"\r\nhello\r\n0\r\n\r\n",
            b"5\r\nhelloXX0\r\n\r\n",
            b"5\nhello\r\n0\r\n\r\n",
            b"00000000000000000005\r\nhello\r\n0\r\n\r\n",
        ] {
            let mut reader = wire;
            assert!(
                matches!(
                    read_chunked_body(&mut reader, Vec::new()).await,
                    Err(ReadError::Malformed(_))
                ),
                "{:?}",
                String::from_utf8_lossy(wire)
            );
        }
    }

    #[tokio::test]
    async fn a_trailer_flood_is_refused() {
        let mut wire = b"0\r\n".to_vec();
        for i in 0..200 {
            wire.extend_from_slice(format!("T{i}: v\r\n").as_bytes());
        }
        wire.extend_from_slice(b"\r\n");
        let mut reader = &wire[..];
        assert!(matches!(
            read_chunked_body(&mut reader, Vec::new()).await,
            Err(ReadError::Malformed(_))
        ));
    }

    #[tokio::test]
    async fn a_chunked_body_over_the_cap_is_refused_before_its_data_is_read() {
        // 0x2000001 = BODY_MAX_BYTES + 1, announced and never sent.
        let mut reader = &b"2000001\r\n"[..];
        assert!(matches!(
            read_chunked_body(&mut reader, Vec::new()).await,
            Err(ReadError::BodyTooLarge)
        ));
        // A chunk just under the cap whose data never arrives is a close, not a
        // cap-sized allocation.
        let wire = b"1ffffff\r\n";
        let mut reader = &wire[..];
        assert!(matches!(
            read_chunked_body(&mut reader, Vec::new()).await,
            Err(ReadError::Closed)
        ));
    }

    #[tokio::test]
    async fn a_chunked_body_cut_short_is_closed() {
        let mut reader = &b"5\r\nhel"[..];
        assert!(matches!(
            read_chunked_body(&mut reader, Vec::new()).await,
            Err(ReadError::Closed)
        ));
    }
}
