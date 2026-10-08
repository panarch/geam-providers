//! One HTTP packet at a time, preserving byte values and the consumption boundary.

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Packet<'input> {
    Request {
        method: &'input [u8],
        uri: Uri<'input>,
        version: (u8, u8),
    },
    Response {
        version: (u8, u8),
        status: u16,
        text: &'input [u8],
    },
    Header {
        field: &'input [u8],
        value: &'input [u8],
    },
    Eoh,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Uri<'input> {
    Origin(&'input [u8]),
    Absolute {
        https: bool,
        host: &'input [u8],
        port: u16,
        path: &'input [u8],
        query_only: bool,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    More,
    Invalid(&'static str),
}

pub(super) fn decode(input: &[u8], headers: bool) -> Result<(Packet<'_>, usize), Error> {
    let end = input
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or(Error::More)?;
    let mut line = &input[..end];
    if let Some(without_cr) = line.strip_suffix(b"\r") {
        line = without_cr;
    }
    if line.iter().any(|byte| *byte == b'\r' || *byte == 0) {
        return Err(Error::Invalid("invalid HTTP line"));
    }
    let packet = if headers {
        header(line)?
    } else {
        start_line(line)?
    };
    Ok((packet, end + 1))
}

fn header(line: &[u8]) -> Result<Packet<'_>, Error> {
    if line.is_empty() {
        return Ok(Packet::Eoh);
    }
    let colon = line
        .iter()
        .position(|byte| *byte == b':')
        .ok_or(Error::Invalid("invalid HTTP header"))?;
    let field = &line[..colon];
    if field.is_empty() || !field.iter().all(|byte| token(*byte)) {
        return Err(Error::Invalid("invalid HTTP header name"));
    }
    let value = &line[colon + 1..];
    if value
        .iter()
        .any(|byte| *byte == 127 || (*byte < 32 && *byte != b'\t'))
    {
        return Err(Error::Invalid("invalid HTTP header value"));
    }
    Ok(Packet::Header {
        field,
        value: value.trim_ascii_start(),
    })
}

fn start_line(line: &[u8]) -> Result<Packet<'_>, Error> {
    if line.starts_with(b"HTTP/") {
        let mut parts = line.splitn(3, |byte| *byte == b' ');
        let version = version(parts.next().unwrap_or_default())?;
        let status = parts.next().ok_or(Error::Invalid("missing HTTP status"))?;
        if status.len() != 3 || !status.iter().all(u8::is_ascii_digit) {
            return Err(Error::Invalid("invalid HTTP status"));
        }
        let status = (status[0] - b'0') as u16 * 100
            + (status[1] - b'0') as u16 * 10
            + (status[2] - b'0') as u16;
        let text = parts.next().unwrap_or_default();
        if text
            .iter()
            .any(|byte| *byte == 127 || (*byte < 32 && *byte != b'\t'))
        {
            return Err(Error::Invalid("invalid HTTP status text"));
        }
        Ok(Packet::Response {
            version,
            status,
            text,
        })
    } else {
        let mut parts = line.split(|byte| *byte == b' ');
        let method = parts.next().unwrap_or_default();
        if method.is_empty() || !method.iter().all(|byte| token(*byte)) {
            return Err(Error::Invalid("invalid HTTP method"));
        }
        let target = parts
            .next()
            .ok_or(Error::Invalid("missing HTTP request target"))?;
        let protocol = parts.next().ok_or(Error::Invalid("missing HTTP version"))?;
        if parts.next().is_some() {
            return Err(Error::Invalid("invalid HTTP request line"));
        }
        Ok(Packet::Request {
            method,
            uri: uri(target)?,
            version: version(protocol)?,
        })
    }
}

fn token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

fn version(value: &[u8]) -> Result<(u8, u8), Error> {
    if let [b'H', b'T', b'T', b'P', b'/', major, b'.', minor] = value
        && major.is_ascii_digit()
        && minor.is_ascii_digit()
    {
        return Ok((major - b'0', minor - b'0'));
    }
    Err(Error::Invalid("invalid HTTP version"))
}

fn uri(target: &[u8]) -> Result<Uri<'_>, Error> {
    if target
        .iter()
        .any(|byte| *byte <= 32 || *byte == 127 || *byte == b'#')
    {
        return Err(Error::Invalid("invalid HTTP request target"));
    }
    if target.starts_with(b"/") {
        return Ok(Uri::Origin(target));
    }
    let (https, rest) = if target.starts_with(b"http://") {
        (false, &target[7..])
    } else if target.starts_with(b"https://") {
        (true, &target[8..])
    } else {
        return Err(Error::Invalid("unsupported HTTP request target"));
    };
    let boundary = rest
        .iter()
        .position(|byte| *byte == b'/' || *byte == b'?')
        .unwrap_or(rest.len());
    let authority = &rest[..boundary];
    let (host, port) = authority_parts(authority, if https { 443 } else { 80 })?;
    let path = &rest[boundary..];
    Ok(Uri::Absolute {
        https,
        host,
        port,
        path,
        query_only: path.starts_with(b"?"),
    })
}

fn authority_parts(authority: &[u8], default: u16) -> Result<(&[u8], u16), Error> {
    if authority.is_empty() || authority.contains(&b'@') {
        return Err(Error::Invalid("invalid HTTP authority"));
    }
    let (host, suffix) = if authority.starts_with(b"[") {
        let end = authority
            .iter()
            .position(|byte| *byte == b']')
            .ok_or(Error::Invalid("invalid IPv6 authority"))?;
        let host = &authority[..=end];
        let inner = std::str::from_utf8(&host[1..end])
            .map_err(|_| Error::Invalid("invalid IPv6 address"))?;
        inner
            .parse::<std::net::Ipv6Addr>()
            .map_err(|_| Error::Invalid("invalid IPv6 address"))?;
        (host, &authority[end + 1..])
    } else {
        let boundary = authority
            .iter()
            .position(|byte| *byte == b':')
            .unwrap_or(authority.len());
        let host = &authority[..boundary];
        if host.is_empty()
            || host
                .iter()
                .any(|byte| *byte >= 127 || b"[]\\".contains(byte))
        {
            return Err(Error::Invalid("invalid HTTP host"));
        }
        (host, &authority[boundary..])
    };
    if suffix.is_empty() {
        return Ok((host, default));
    }
    let digits = suffix
        .strip_prefix(b":")
        .ok_or(Error::Invalid("invalid HTTP port"))?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(Error::Invalid("invalid HTTP port"));
    }
    let port = digits
        .iter()
        .try_fold(0u16, |port, digit| {
            port.checked_mul(10)?.checked_add((digit - b'0') as u16)
        })
        .ok_or(Error::Invalid("invalid HTTP port"))?;
    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::{Error, Packet, Uri, decode};

    #[test]
    fn one_packet_retains_exact_consumption_raw_fields_and_status_text() {
        let input = b"POST /a?q=%FF HTTP/1.1\r\nHeader: v\r\n\r\nbody\xff";
        assert_eq!(
            decode(input, false),
            Ok((
                Packet::Request {
                    method: b"POST",
                    uri: Uri::Origin(b"/a?q=%FF"),
                    version: (1, 1)
                },
                24
            ))
        );
        assert_eq!(&input[24..], b"Header: v\r\n\r\nbody\xff");
        assert_eq!(
            decode(b"X: \t\xff\x80 \t\r\nbody", true),
            Ok((
                Packet::Header {
                    field: b"X",
                    value: b"\xff\x80 \t"
                },
                10
            ))
        );
        assert_eq!(
            decode(b"X:\nnext", true),
            Ok((
                Packet::Header {
                    field: b"X",
                    value: b""
                },
                3
            ))
        );
        assert_eq!(decode(b"\r\nbody", true), Ok((Packet::Eoh, 2)));
        assert_eq!(decode(b"\n", true), Ok((Packet::Eoh, 1)));
        assert_eq!(
            decode(b"HTTP/1.0 200 OK\r\nrest", false),
            Ok((
                Packet::Response {
                    version: (1, 0),
                    status: 200,
                    text: b"OK"
                },
                17
            ))
        );
        assert_eq!(
            decode(b"HTTP/1.1 204\n", false),
            Ok((
                Packet::Response {
                    version: (1, 1),
                    status: 204,
                    text: b""
                },
                13
            ))
        );
        assert_eq!(
            decode(b"HTTP/1.1 200 \t\xff\n", false),
            Ok((
                Packet::Response {
                    version: (1, 1),
                    status: 200,
                    text: b"\t\xff"
                },
                16
            ))
        );
    }

    #[test]
    fn incomplete_and_invalid_packets_have_exact_failure_owners() {
        for bytes in [&b""[..], b"GET /", b"X: v\r"] {
            assert_eq!(decode(bytes, false), Err(Error::More));
        }
        for (bytes, headers, reason) in [
            (&b"GET / HTTP/1.1\rX\n"[..], false, "invalid HTTP line"),
            (b"X: \0\n", true, "invalid HTTP line"),
            (b"no colon\n", true, "invalid HTTP header"),
            (b": empty\n", true, "invalid HTTP header name"),
            (b"Bad Header: v\n", true, "invalid HTTP header name"),
            (b"X: \x7f\n", true, "invalid HTTP header value"),
            (b"X: \x0c\n", true, "invalid HTTP header value"),
            (b"HTTP/1.1\n", false, "missing HTTP status"),
            (b"HTTP/x.1 200 OK\n", false, "invalid HTTP version"),
            (b"HTTP/1.01 200 OK\n", false, "invalid HTTP version"),
            (b"HTTP/1.1 20 OK\n", false, "invalid HTTP status"),
            (b"HTTP/1.1 2x0 OK\n", false, "invalid HTTP status"),
            (b"HTTP/1.1 200 \x01\n", false, "invalid HTTP status text"),
            (b"HTTP/1.1 200 \x7f\n", false, "invalid HTTP status text"),
            (b" / HTTP/1.1\n", false, "invalid HTTP method"),
            (b"G\x80T / HTTP/1.1\n", false, "invalid HTTP method"),
            (b"GET\n", false, "missing HTTP request target"),
            (b"GET /\n", false, "missing HTTP version"),
            (b"GET / HTTP/1.1 more\n", false, "invalid HTTP request line"),
            (b"GET / HTP/1.1\n", false, "invalid HTTP version"),
            (b"GET / HTTP/x.1\n", false, "invalid HTTP version"),
            (b"GET / HTTP/1.x\n", false, "invalid HTTP version"),
            (
                b"GET /#fragment HTTP/1.1\n",
                false,
                "invalid HTTP request target",
            ),
            (
                b"GET /\x01 HTTP/1.1\n",
                false,
                "invalid HTTP request target",
            ),
            (
                b"GET /\x7f HTTP/1.1\n",
                false,
                "invalid HTTP request target",
            ),
            (
                b"GET * HTTP/1.1\n",
                false,
                "unsupported HTTP request target",
            ),
            (
                b"CONNECT host:443 HTTP/1.1\n",
                false,
                "unsupported HTTP request target",
            ),
            (
                b"GET ftp://host/ HTTP/1.1\n",
                false,
                "unsupported HTTP request target",
            ),
            (b"GET http:/// HTTP/1.1\n", false, "invalid HTTP authority"),
            (
                b"GET http://user@host/ HTTP/1.1\n",
                false,
                "invalid HTTP authority",
            ),
            (
                b"GET http://[::1/ HTTP/1.1\n",
                false,
                "invalid IPv6 authority",
            ),
            (
                b"GET http://[bad]/ HTTP/1.1\n",
                false,
                "invalid IPv6 address",
            ),
            (
                b"GET http://[\xff]/ HTTP/1.1\n",
                false,
                "invalid IPv6 address",
            ),
            (b"GET http://:80/ HTTP/1.1\n", false, "invalid HTTP host"),
            (b"GET http://bad]/ HTTP/1.1\n", false, "invalid HTTP host"),
            (b"GET http://\xff/ HTTP/1.1\n", false, "invalid HTTP host"),
            (
                b"GET http://[::1]oops/ HTTP/1.1\n",
                false,
                "invalid HTTP port",
            ),
            (b"GET http://host:/ HTTP/1.1\n", false, "invalid HTTP port"),
            (b"GET http://host:x/ HTTP/1.1\n", false, "invalid HTTP port"),
            (
                b"GET http://host:999999/ HTTP/1.1\n",
                false,
                "invalid HTTP port",
            ),
        ] {
            assert_eq!(
                decode(bytes, headers),
                Err(Error::Invalid(reason)),
                "{bytes:?}"
            );
        }
    }

    #[test]
    fn absolute_uri_keeps_brackets_ports_queries_and_percent_encoding() {
        for (target, expected) in [
            (
                &b"http://host"[..],
                Uri::Absolute {
                    https: false,
                    host: b"host",
                    port: 80,
                    path: b"",
                    query_only: false,
                },
            ),
            (
                b"https://host?q=%20",
                Uri::Absolute {
                    https: true,
                    host: b"host",
                    port: 443,
                    path: b"?q=%20",
                    query_only: true,
                },
            ),
            (
                b"https://[::1]:8443/a?x=1",
                Uri::Absolute {
                    https: true,
                    host: b"[::1]",
                    port: 8443,
                    path: b"/a?x=1",
                    query_only: false,
                },
            ),
            (
                b"http://host:0/",
                Uri::Absolute {
                    https: false,
                    host: b"host",
                    port: 0,
                    path: b"/",
                    query_only: false,
                },
            ),
            (
                b"http://host:65535/",
                Uri::Absolute {
                    https: false,
                    host: b"host",
                    port: 65535,
                    path: b"/",
                    query_only: false,
                },
            ),
        ] {
            assert_eq!(super::uri(target), Ok(expected));
        }
        assert_eq!(
            super::authority_parts(b"host:65536", 80),
            Err(Error::Invalid("invalid HTTP port"))
        );
        assert_eq!(
            super::uri(b"/path with-space"),
            Err(Error::Invalid("invalid HTTP request target"))
        );
        assert!(super::token(b'!'));
    }
}
