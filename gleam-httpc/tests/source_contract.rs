#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::host::HostComponentProfile;
use geam_httpc::Component;
use geam_httpc::transport::{self, Failure, Request, Response, Transport};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncReadExt;

#[derive(Default)]
struct RecordingTransport {
    requests: Mutex<Vec<Request>>,
}

impl Transport for RecordingTransport {
    fn send(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Failure>> + Send + '_>> {
        self.requests.lock().unwrap().push(request.clone());
        Box::pin(async move {
            if request.url.ends_with("/invalid-method") {
                return Err(Failure::InvalidRequest("invalid HTTP method".into()));
            }
            if request.url.ends_with("/timeout") {
                return Err(Failure::Timeout);
            }
            if request.url.ends_with("/posix") {
                return Err(Failure::FailedToConnect {
                    ip4: transport::ConnectError::Posix("econnrefused".into()),
                    ip6: transport::ConnectError::Posix("enetunreach".into()),
                });
            }
            if request.url.ends_with("/tls") {
                let alert = transport::ConnectError::TlsAlert {
                    code: "unknown_ca".into(),
                    detail: "certificate rejected".into(),
                };
                return Err(Failure::FailedToConnect {
                    ip4: alert.clone(),
                    ip6: alert,
                });
            }
            Ok(Response {
                version: "HTTP/1.1".into(),
                status: 200,
                reason: "OK".into(),
                headers: if request.url.ends_with("/options") {
                    vec![
                        ("x-duplicate".into(), "first".into()),
                        ("x-duplicate".into(), "second".into()),
                    ]
                } else {
                    vec![]
                },
                body: if request.url.ends_with("/invalid-utf8") {
                    vec![255]
                } else if request.url.ends_with("/binary") {
                    vec![0, 255]
                } else {
                    b"hello".to_vec()
                },
            })
        })
    }
}

#[test]
fn original_httpc_methods_options_binary_and_errors_use_typed_host_boundary() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let (mut module, functions) =
        geam_bindings::load::<Vec<geam::gleam_stdlib::IoOutput>>().unwrap();
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: geam::gleam_erlang::Configuration::default(),
        gleam_httpc: HostProviderConfiguration::empty(),
    }
    .initialize()
    .unwrap();
    let transport = Arc::new(RecordingTransport::default());
    <geam_bindings::Profile<Vec<geam::gleam_stdlib::IoOutput>> as HostComponentProfile<
        Component<geam_bindings::Profile<Vec<geam::gleam_stdlib::IoOutput>>>,
    >>::component_state(&mut state)
    .set_transport(transport.clone());
    let mut echo = Vec::new();
    executor.block_on(async {
        macro_rules! assert_case {
            ($function:expr, $url:literal) => {{
                let url = $url;
                let result = module
                    .with_execution(&host, &mut state, &mut echo, async |scope| {
                        scope.call($function, (url.into(),)).await
                    })
                    .await
                    .unwrap()
                    .unwrap();
                assert!(result, "original gleam_httpc contract: {url}");
            }};
        }
        assert_case!(&functions.methods, "http://fixture.invalid/methods");
        assert_case!(
            &functions.options_and_headers,
            "http://fixture.invalid/options"
        );
        assert_case!(&functions.binary_response, "http://fixture.invalid/binary");
        assert_case!(
            &functions.invalid_utf8,
            "http://fixture.invalid/invalid-utf8"
        );
        assert_case!(&functions.timeout, "http://fixture.invalid/timeout");
        assert_case!(&functions.posix_error, "http://fixture.invalid/posix");
        assert_case!(&functions.tls_error, "http://fixture.invalid/tls");
        macro_rules! assert_host_failure {
            ($function:expr, $url:literal, $message:literal) => {{
                let outcome = module
                    .with_execution(&host, &mut state, &mut echo, async |scope| {
                        scope.call($function, ($url.into(),)).await
                    })
                    .await;
                let diagnostic = format!("{outcome:?}");
                assert!(diagnostic.contains($message), "{diagnostic}");
            }};
        }
        assert_host_failure!(
            &functions.negative_timeout,
            "http://fixture.invalid/negative-timeout",
            "non-negative millisecond"
        );
        assert_host_failure!(
            &functions.negative_timeout_body,
            "http://fixture.invalid/negative-timeout-body",
            "non-negative millisecond"
        );
        assert_host_failure!(
            &functions.unaligned_request,
            "http://fixture.invalid/unaligned",
            "byte aligned"
        );
        assert_host_failure!(
            &functions.invalid_method,
            "http://fixture.invalid/invalid-method",
            "invalid HTTP method"
        );
    });
    let requests = transport.requests.lock().unwrap();
    assert_eq!(requests.len(), 17);
    assert_eq!(
        requests[..10]
            .iter()
            .map(|request| request.method.as_str())
            .collect::<Vec<_>>(),
        [
            "GET", "HEAD", "OPTIONS", "POST", "PUT", "DELETE", "TRACE", "CONNECT", "PATCH",
            "REPORT"
        ]
    );
    for request in &requests[..3] {
        assert!(request.body.is_none());
        assert!(request.content_type.is_none());
        assert!(request.verify_tls);
        assert!(!request.follow_redirects);
        assert_eq!(request.timeout, Duration::from_secs(30));
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "user-agent" && value == "gleam_httpc/5.0.0")
        );
    }
    assert_eq!(requests[3].body.as_deref(), Some(b"post".as_slice()));
    assert_eq!(
        requests[3].content_type.as_deref(),
        Some("application/octet-stream")
    );
    let configured = &requests[10];
    assert!(!configured.verify_tls);
    assert!(configured.follow_redirects);
    assert_eq!(configured.timeout, Duration::from_millis(123));
    assert_eq!(configured.content_type.as_deref(), Some("text/plain"));
    assert_eq!(configured.body.as_deref(), Some(b"payload".as_slice()));
    assert_eq!(
        configured
            .headers
            .iter()
            .filter(|(name, _)| name == "x-duplicate")
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    assert_eq!(requests[11].body.as_deref(), Some([0, 255].as_slice()));
}

#[test]
fn cancelling_original_gleam_send_releases_the_pending_socket() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let (mut module, functions) =
        geam_bindings::load::<Vec<geam::gleam_stdlib::IoOutput>>().unwrap();
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: geam::gleam_erlang::Configuration::default(),
        gleam_httpc: HostProviderConfiguration::empty(),
    }
    .initialize()
    .unwrap();
    let mut echo = Vec::new();
    executor.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let (headers_sent, mut headers_received) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut received = Vec::new();
            while !received.windows(4).any(|part| part == b"\r\n\r\n") {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).await.unwrap();
                assert!(count > 0, "Gleam request closed before headers");
                received.extend_from_slice(&buffer[..count]);
            }
            assert!(received.starts_with(b"GET /pending HTTP/1.1\r\n"));
            headers_sent.send(()).unwrap();
            let mut byte = [0];
            stream.read(&mut byte).await
        });
        let url = format!("http://{address}/pending");
        let mut pending = Box::pin(module.with_execution(&host, &mut state, &mut echo, async |scope| {
            scope.call(&functions.verify, (url.into(),)).await
        }));
        tokio::select! {
            result = &mut pending => panic!("request finished before server replied: {result:?}"),
            observed = &mut headers_received => observed.unwrap(),
        }
        drop(pending);
        let closed = tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .expect("cancelled Geam call releases socket")
            .unwrap();
        assert!(
            matches!(closed, Ok(0))
                || matches!(closed, Err(ref error) if error.kind() == std::io::ErrorKind::ConnectionReset),
            "expected EOF or reset after cancellation, got {closed:?}"
        );
    });
}
