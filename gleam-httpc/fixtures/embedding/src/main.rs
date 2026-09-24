#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: geam::gleam_erlang::Configuration::default(),
        gleam_httpc: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();
    executor.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut request = Vec::new();
            loop {
                let mut chunk = [0; 1024];
                let count = stream.read(&mut chunk).await?;
                if count == 0 {
                    return Err(std::io::Error::other("client closed before headers"));
                }
                request.extend_from_slice(&chunk[..count]);
                if request.windows(4).any(|part| part == b"\r\n\r\n") {
                    break;
                }
            }
            if !request.starts_with(b"GET /hello HTTP/1.1\r\n") {
                return Err(std::io::Error::other("unexpected request"));
            }
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
                )
                .await?;
            Ok(())
        });
        let url = format!("http://{address}/hello");
        let matched = module
            .with_execution(&host, &mut state, &mut echo, async |scope| {
                scope.call(&functions.verify, (url.into(),)).await
            })
            .await??;
        assert!(matched, "original gleam_httpc receives the HTTP response");
        server.await??;
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}
