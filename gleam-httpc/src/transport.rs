use reqwest::{Certificate, Client, Method, redirect};
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

#[derive(Clone, Debug)]
/// One HTTP request prepared by the unchanged Gleam package.
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub content_type: Option<String>,
    pub body: Option<Vec<u8>>,
    pub verify_tls: bool,
    pub follow_redirects: bool,
    pub timeout: Duration,
}

#[derive(Debug)]
/// Response data returned to the unchanged Gleam package.
pub struct Response {
    pub version: String,
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A native connection failure in the upstream `ConnectError` vocabulary.
pub enum ConnectError {
    Posix(String),
    TlsAlert { code: String, detail: String },
}

#[derive(Debug, PartialEq, Eq)]
/// Transport failure mapped through the original `gleam_httpc` FFI boundary.
pub enum Failure {
    Timeout,
    FailedToConnect {
        ip4: ConnectError,
        ip6: ConnectError,
    },
    InvalidRequest(String),
}

/// Host-owned asynchronous network capability used by the provider run state.
/// Dropping the returned future must cancel outstanding request work; an
/// implementation must not leave a detached socket or body task behind.
pub trait Transport: Send + Sync {
    fn send(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Failure>> + Send + '_>>;
}

pub(crate) struct HttpTransport {
    verified_no_redirect: Client,
    verified_redirect: Client,
    unverified_no_redirect: Client,
    unverified_redirect: Client,
}

type ClientBuilder = dyn Fn(bool, bool, &[Certificate]) -> Result<Client, String>;

impl HttpTransport {
    pub fn new(root_certificate_pem: Option<&str>) -> Result<Self, String> {
        let certificates = match root_certificate_pem {
            Some(pem) => {
                let certificates = Certificate::from_pem_bundle(pem.as_bytes())
                    .map_err(|error| error.to_string())?;
                if certificates.is_empty() {
                    return Err("root_certificate_pem contains no certificate".to_owned());
                }
                certificates
            }
            None => Vec::new(),
        };
        Self::build_clients(&certificates, &build_client)
    }

    fn build_clients(certificates: &[Certificate], build: &ClientBuilder) -> Result<Self, String> {
        Ok(Self {
            verified_no_redirect: build(true, false, certificates)?,
            verified_redirect: build(true, true, certificates)?,
            unverified_no_redirect: build(false, false, certificates)?,
            unverified_redirect: build(false, true, certificates)?,
        })
    }

    fn client(&self, verify_tls: bool, follow_redirects: bool) -> &Client {
        match (verify_tls, follow_redirects) {
            (true, false) => &self.verified_no_redirect,
            (true, true) => &self.verified_redirect,
            (false, false) => &self.unverified_no_redirect,
            (false, true) => &self.unverified_redirect,
        }
    }
}

fn build_client(
    verify_tls: bool,
    follow_redirects: bool,
    certificates: &[Certificate],
) -> Result<Client, String> {
    let mut builder = Client::builder()
        .http1_only()
        .no_proxy()
        .redirect(if follow_redirects {
            redirect::Policy::limited(10)
        } else {
            redirect::Policy::none()
        })
        .tls_danger_accept_invalid_certs(!verify_tls);
    for certificate in certificates {
        builder = builder.add_root_certificate(certificate.clone());
    }
    builder.build().map_err(|error| error.to_string())
}

impl Transport for HttpTransport {
    fn send(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Failure>> + Send + '_>> {
        Box::pin(async move {
            let method = Method::from_bytes(request.method.as_bytes())
                .map_err(|error| Failure::InvalidRequest(error.to_string()))?;
            let client = self.client(request.verify_tls, request.follow_redirects);
            let mut pending = client
                .request(method, &request.url)
                .timeout(request.timeout);
            for (name, value) in &request.headers {
                pending = pending.header(name.as_str(), value.as_str());
            }
            if let Some(content_type) = &request.content_type
                && !request
                    .headers
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            {
                pending = pending.header("content-type", content_type.as_str());
            }
            if let Some(body) = request.body {
                pending = pending.body(body);
            }
            let response = pending.send().await.map_err(request_failure)?;
            let status = response.status();
            let version = format!("{:?}", response.version());
            let reason = status.canonical_reason().unwrap_or("").to_owned();
            let headers = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    let text = value
                        .as_bytes()
                        .iter()
                        .map(|byte| char::from(*byte))
                        .collect();
                    (name.as_str().to_owned(), text)
                })
                .collect();
            let body = response.bytes().await.map_err(request_failure)?.to_vec();
            Ok(Response {
                version,
                status: status.as_u16(),
                reason,
                headers,
                body,
            })
        })
    }
}

fn request_failure(error: reqwest::Error) -> Failure {
    if error.is_timeout() {
        return Failure::Timeout;
    }
    if !error.is_connect() {
        return Failure::InvalidRequest(error.to_string());
    }
    let failure = if let Some(tls) = find_error::<rustls::Error>(&error) {
        ConnectError::TlsAlert {
            code: tls_code(tls).to_owned(),
            detail: tls.to_string(),
        }
    } else {
        ConnectError::Posix(
            posix_code(innermost_io_error(&error).map(std::io::Error::kind)).to_owned(),
        )
    };
    Failure::FailedToConnect {
        ip4: failure.clone(),
        ip6: failure,
    }
}

fn tls_code(error: &rustls::Error) -> &'static str {
    match error {
        rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer) => "unknown_ca",
        rustls::Error::InvalidCertificate(_) => "bad_certificate",
        _ => "handshake_failure",
    }
}

fn posix_code(kind: Option<std::io::ErrorKind>) -> &'static str {
    match kind {
        Some(std::io::ErrorKind::ConnectionRefused) => "econnrefused",
        Some(std::io::ErrorKind::ConnectionReset) => "econnreset",
        Some(std::io::ErrorKind::HostUnreachable) => "ehostunreach",
        Some(std::io::ErrorKind::NetworkUnreachable) => "enetunreach",
        Some(std::io::ErrorKind::NotFound) => "nxdomain",
        _ => "eio",
    }
}

fn find_error<'a, E: Error + 'static>(error: &'a (dyn Error + 'static)) -> Option<&'a E> {
    if let Some(found) = error.downcast_ref::<E>() {
        return Some(found);
    }
    if let Some(io) = error.downcast_ref::<std::io::Error>()
        && let Some(found) = io.get_ref().and_then(|inner| find_error::<E>(inner))
    {
        return Some(found);
    }
    error.source().and_then(find_error::<E>)
}

fn innermost_io_error<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a std::io::Error> {
    let io = error.downcast_ref::<std::io::Error>();
    if let Some(inner) = io.and_then(std::io::Error::get_ref)
        && let Some(found) = innermost_io_error(inner)
    {
        return Some(found);
    }
    error.source().and_then(innermost_io_error).or(io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::{
        BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa,
        KeyPair, KeyUsagePurpose,
    };
    use reqwest::dns::{Name, Resolve, Resolving};
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
    use std::sync::Arc;
    use std::time::SystemTime;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;

    trait TestStream: AsyncRead + AsyncWrite + Unpin + Send {}

    impl<T: AsyncRead + AsyncWrite + Unpin + Send> TestStream for T {}

    #[test]
    fn tls_and_posix_codes_preserve_distinct_failure_families() {
        assert_eq!(
            tls_code(&rustls::Error::InvalidCertificate(
                rustls::CertificateError::UnknownIssuer,
            )),
            "unknown_ca"
        );
        assert_eq!(
            tls_code(&rustls::Error::InvalidCertificate(
                rustls::CertificateError::BadEncoding,
            )),
            "bad_certificate"
        );
        assert_eq!(
            tls_code(&rustls::Error::General("handshake".into())),
            "handshake_failure"
        );
        for (kind, expected) in [
            (Some(std::io::ErrorKind::ConnectionRefused), "econnrefused"),
            (Some(std::io::ErrorKind::ConnectionReset), "econnreset"),
            (Some(std::io::ErrorKind::HostUnreachable), "ehostunreach"),
            (Some(std::io::ErrorKind::NetworkUnreachable), "enetunreach"),
            (Some(std::io::ErrorKind::NotFound), "nxdomain"),
            (Some(std::io::ErrorKind::Other), "eio"),
            (None, "eio"),
        ] {
            assert_eq!(posix_code(kind), expected);
        }
        let nested = std::io::Error::other(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "reset",
        ));
        assert_eq!(
            innermost_io_error(&nested).map(std::io::Error::kind),
            Some(std::io::ErrorKind::ConnectionReset)
        );
        assert!(find_error::<std::io::Error>(&nested).is_some());
    }

    #[test]
    fn every_client_build_failure_prevents_transport_initialization() {
        for rejected in [(true, false), (true, true), (false, false), (false, true)] {
            let result =
                HttpTransport::build_clients(&[], &move |verify_tls, follow_redirects, roots| {
                    if (verify_tls, follow_redirects) == rejected {
                        Err(format!("client build failed for {rejected:?}"))
                    } else {
                        build_client(verify_tls, follow_redirects, roots)
                    }
                });
            assert_eq!(
                result.err(),
                Some(format!("client build failed for {rejected:?}"))
            );
        }
    }

    fn http_request(url: String) -> Request {
        Request {
            method: "GET".into(),
            url,
            headers: vec![("user-agent".into(), "fixture-agent".into())],
            content_type: None,
            body: None,
            verify_tls: true,
            follow_redirects: false,
            timeout: Duration::from_secs(2),
        }
    }

    async fn respond(
        mut stream: Box<dyn TestStream>,
        observed: Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>,
    ) {
        let mut received = Vec::new();
        while !received.windows(4).any(|window| window == b"\r\n\r\n") {
            let mut buffer = [0; 1024];
            let count = stream.read(&mut buffer).await.expect("request read");
            if count == 0 {
                return;
            }
            received.extend_from_slice(&buffer[..count]);
        }
        let header_end = received
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        let request = String::from_utf8_lossy(&received[..header_end]).into_owned();
        let content_length = request
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .unwrap_or(0);
        let missing = (header_end + content_length).saturating_sub(received.len());
        let mut body_suffix = vec![0; missing];
        stream
            .read_exact(&mut body_suffix)
            .await
            .expect("complete request body");
        received.extend_from_slice(&body_suffix);
        if let Some(observed) = observed {
            observed.send(received).expect("request observer");
        }
        if request.contains(" /slow-body ") {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n")
                .await
                .expect("response headers");
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = stream.write_all(b"hello").await;
            return;
        }
        let response: &[u8] = if request.contains(" /redirect ") {
            b"HTTP/1.1 302 Found\r\nLocation: /hello\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        } else if request.contains(" /partial ") {
            b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nshort"
        } else if request.contains(" /wait ") {
            tokio::time::sleep(Duration::from_millis(100)).await;
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nX-Duplicate: first\r\nX-Duplicate: second\r\nConnection: close\r\n\r\nhello"
        };
        stream.write_all(response).await.expect("response write");
        stream.shutdown().await.expect("response shutdown");
    }

    #[test]
    fn http_transport_preserves_request_and_response_policy() {
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        executor.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let (observed_tx, mut observed_rx) = tokio::sync::mpsc::unbounded_channel();
            let server = tokio::spawn(async move {
                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    let observed_tx = observed_tx.clone();
                    tokio::spawn(async move { respond(Box::new(stream), Some(observed_tx)).await });
                }
            });
            let transport = HttpTransport::new(None).unwrap();
            let base = format!("http://{address}");

            let response = transport
                .send(http_request(format!("{base}/hello")))
                .await
                .unwrap();
            assert_eq!(response.version, "HTTP/1.1");
            assert_eq!(response.status, 200);
            assert_eq!(response.reason, "OK");
            assert_eq!(response.body, b"hello");
            assert_eq!(
                response
                    .headers
                    .iter()
                    .filter(|(name, _)| name == "x-duplicate")
                    .map(|(_, value)| value.as_str())
                    .collect::<Vec<_>>(),
                ["first", "second"]
            );

            let stopped = transport
                .send(http_request(format!("{base}/redirect")))
                .await
                .unwrap();
            assert_eq!(stopped.status, 302);
            let mut following = http_request(format!("{base}/redirect"));
            following.follow_redirects = true;
            let followed = transport.send(following).await.unwrap();
            assert_eq!(followed.status, 200);
            assert_eq!(followed.body, b"hello");

            let mut post = http_request(format!("{base}/hello"));
            post.method = "POST".into();
            post.content_type = Some("application/octet-stream".into());
            post.body = Some(vec![0, 255]);
            assert_eq!(transport.send(post).await.unwrap().status, 200);
            let mut content_type = http_request(format!("{base}/hello"));
            content_type
                .headers
                .push(("content-type".into(), "text/plain".into()));
            content_type.content_type = Some("text/plain".into());
            content_type.body = Some(b"body".to_vec());
            assert_eq!(transport.send(content_type).await.unwrap().status, 200);
            let mut duplicate = http_request(format!("{base}/hello"));
            duplicate.headers.push(("x-repeat".into(), "first".into()));
            duplicate.headers.push(("x-repeat".into(), "second".into()));
            assert_eq!(transport.send(duplicate).await.unwrap().status, 200);

            let mut invalid_method = http_request(format!("{base}/hello"));
            invalid_method.method = "bad method".into();
            assert_eq!(
                std::mem::discriminant(&transport.send(invalid_method).await.unwrap_err()),
                std::mem::discriminant(&Failure::InvalidRequest(String::new()))
            );
            let invalid_url = http_request("not a URL".into());
            assert_eq!(
                std::mem::discriminant(&transport.send(invalid_url).await.unwrap_err()),
                std::mem::discriminant(&Failure::InvalidRequest(String::new()))
            );
            let mut invalid_header = http_request(format!("{base}/hello"));
            invalid_header
                .headers
                .push(("invalid name".into(), "value".into()));
            assert_eq!(
                std::mem::discriminant(&transport.send(invalid_header).await.unwrap_err()),
                std::mem::discriminant(&Failure::InvalidRequest(String::new()))
            );

            let partial = transport
                .send(http_request(format!("{base}/partial")))
                .await;
            assert_eq!(
                std::mem::discriminant(&partial.unwrap_err()),
                std::mem::discriminant(&Failure::InvalidRequest(String::new()))
            );
            let mut waiting = http_request(format!("{base}/wait"));
            waiting.timeout = Duration::from_millis(10);
            assert_eq!(transport.send(waiting).await.unwrap_err(), Failure::Timeout);
            let mut slow_body = http_request(format!("{base}/slow-body"));
            slow_body.timeout = Duration::from_millis(10);
            assert_eq!(
                transport.send(slow_body).await.unwrap_err(),
                Failure::Timeout
            );

            let completed_wait = transport
                .send(http_request(format!("{base}/wait")))
                .await
                .unwrap();
            assert_eq!(completed_wait.status, 200);
            let (first, second) = tokio::join!(
                transport.send(http_request(format!("{base}/hello"))),
                transport.send(http_request(format!("{base}/hello")))
            );
            assert_eq!(first.unwrap().body, b"hello");
            assert_eq!(second.unwrap().body, b"hello");
            for method in ["HEAD", "OPTIONS", "REPORT"] {
                let mut request = http_request(format!("{base}/hello"));
                request.method = method.to_owned();
                let response = transport.send(request).await.unwrap();
                assert_eq!(response.status, 200);
                let expected: &[u8] = if method == "HEAD" { b"" } else { b"hello" };
                assert_eq!(response.body, expected);
            }

            let observed: Vec<_> = std::iter::from_fn(|| observed_rx.try_recv().ok()).collect();
            assert_eq!(observed.len(), 16);
            let first = &observed[0];
            assert!(first.starts_with(b"GET /hello HTTP/1.1\r\n"));
            assert!(
                first
                    .windows(25)
                    .any(|part| part == b"user-agent: fixture-agent")
            );
            let post = &observed[4];
            assert!(post.starts_with(b"POST /hello HTTP/1.1\r\n"));
            assert!(
                post.windows(38)
                    .any(|part| part == b"content-type: application/octet-stream")
            );
            assert!(post.ends_with(b"\r\n\r\n\x00\xff"));
            let explicit = &observed[5];
            assert!(
                explicit
                    .windows(24)
                    .any(|part| part == b"content-type: text/plain")
            );
            assert_eq!(
                String::from_utf8_lossy(explicit)
                    .matches("content-type: text/plain")
                    .count(),
                1
            );
            let repeated = String::from_utf8_lossy(&observed[6]);
            assert!(repeated.contains("x-repeat: first\r\n"));
            assert!(repeated.contains("x-repeat: second\r\n"));

            server.abort();
            assert!(server.await.unwrap_err().is_cancelled());
        });
    }

    #[test]
    fn local_server_stops_when_client_closes_before_headers() {
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        executor.block_on(async {
            let (client, server) = tokio::io::duplex(64);
            drop(client);
            respond(Box::new(server), None).await;
        });
    }

    #[test]
    fn tls_transport_uses_explicit_ca_and_respects_verification_override() {
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        executor.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let now = SystemTime::now();
            let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
            ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
            ca_params.not_before = (now - Duration::from_secs(86_400)).into();
            ca_params.not_after = (now + Duration::from_secs(86_400 * 365)).into();
            let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();
            let mut server_params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
            server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
            server_params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
            server_params.not_before = (now - Duration::from_secs(86_400)).into();
            server_params.not_after = (now + Duration::from_secs(86_400 * 30)).into();
            let server_key = KeyPair::generate().unwrap();
            let server_cert = server_params.signed_by(&server_key, &ca).unwrap();
            let certificate = CertificateDer::from(server_cert.der().to_vec());
            let key = PrivateKeyDer::from_pem_slice(server_key.serialize_pem().as_bytes()).unwrap();
            let config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![certificate], key)
                .unwrap();
            let acceptor = TlsAcceptor::from(Arc::new(config));
            let server = tokio::spawn(async move {
                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    let acceptor = acceptor.clone();
                    tokio::spawn(async move {
                        if let Ok(stream) = acceptor.accept(stream).await {
                            respond(Box::new(stream), None).await;
                        }
                    });
                }
            });
            let url = format!("https://localhost:{}/hello", address.port());
            let trusted = HttpTransport::new(Some(&ca.pem())).unwrap();
            assert_eq!(
                trusted.send(http_request(url.clone())).await.unwrap().body,
                b"hello"
            );
            let untrusted = HttpTransport::new(None).unwrap();
            assert!(matches!(
                untrusted.send(http_request(url.clone())).await,
                Err(Failure::FailedToConnect { ip4: ConnectError::TlsAlert { code, .. }, .. })
                if code == "unknown_ca"
            ));
            let mut explicitly_unverified = http_request(url.clone());
            explicitly_unverified.verify_tls = false;
            assert_eq!(
                untrusted.send(explicitly_unverified).await.unwrap().body,
                b"hello"
            );
            let mut unverified_redirect = http_request(url.replace("/hello", "/redirect"));
            unverified_redirect.verify_tls = false;
            unverified_redirect.follow_redirects = true;
            assert_eq!(
                untrusted.send(unverified_redirect).await.unwrap().body,
                b"hello"
            );
            server.abort();
            assert!(server.await.unwrap_err().is_cancelled());
        });
    }

    #[test]
    fn refused_socket_maps_to_connection_error() {
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        executor.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            drop(listener);
            let transport = HttpTransport::new(None).unwrap();
            assert!(matches!(
                transport.send(http_request(format!("http://{address}/hello"))).await,
                Err(Failure::FailedToConnect { ip4: ConnectError::Posix(code), .. })
                if code == "econnrefused"
            ));
        });
    }

    struct MissingDns;

    impl Resolve for MissingDns {
        fn resolve(&self, name: Name) -> Resolving {
            assert_eq!(name.as_str(), "fixture.invalid");
            Box::pin(async {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "fixture host does not exist",
                )) as Box<dyn Error + Send + Sync>)
            })
        }
    }

    #[test]
    fn resolver_failure_maps_to_a_connection_error_without_external_dns() {
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        executor.block_on(async {
            let client = Client::builder()
                .no_proxy()
                .dns_resolver(MissingDns)
                .build()
                .unwrap();
            let error = client
                .get("http://fixture.invalid/")
                .send()
                .await
                .unwrap_err();
            assert_eq!(
                request_failure(error),
                Failure::FailedToConnect {
                    ip4: ConnectError::Posix("nxdomain".into()),
                    ip6: ConnectError::Posix("nxdomain".into()),
                }
            );
        });
    }
}
