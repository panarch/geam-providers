// Generated bindings also expose state accessors unused by this streaming host.
#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;
use geam::gleam_stdlib::{GleamStdlibRunState, IoOutput, IoSink, IoStream};

struct Streams;
impl IoSink for Streams {
    fn emit(&mut self, output: IoOutput) {
        use std::io::Write;
        match output.stream() {
            IoStream::Stdout => {
                std::io::stdout()
                    .write_all(output.text().as_bytes())
                    .unwrap();
                std::io::stdout().flush().unwrap();
            }
            IoStream::Stderr => {
                std::io::stderr()
                    .write_all(output.text().as_bytes())
                    .unwrap();
                std::io::stderr().flush().unwrap();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let certificate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../certs")
        .canonicalize()?;
    let certificate = certificate_root
        .join("cert.pem")
        .to_string_lossy()
        .into_owned();
    let key = certificate_root
        .join("key.pem")
        .to_string_lossy()
        .into_owned();
    // The glisten capability owns its reactor. The execution host needs time only.
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let mut modules = Vec::new();
    if std::env::args().nth(1).as_deref() != Some("--prepared") {
        let program = geam_bindings::project::<Streams>().compile()?;
        let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
        modules.push((bindings.seal()?, functions));
    }
    modules.push(geam_bindings::load::<Streams>()?);
    for (mut module, functions) in modules {
        let mut state = geam_bindings::RunStateInputs {
            stdlib: GleamStdlibRunState::from_seed_with_io([0; 32], Streams),
            erlang: Default::default(),
            argv: HostProviderConfiguration::empty(),
            gleam_otp: HostProviderConfiguration::empty(),
            glisten: HostProviderConfiguration::empty(),
            logging: HostProviderConfiguration::empty(),
        }
        .initialize()?;
        let mut echo = Vec::new();
        executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    scope
                        .call(
                            &functions.verify,
                            (certificate.clone().into(), key.clone().into()),
                        )
                        .await
                }),
            )?
            .try_into_value()
            .expect("fixture must return normally")?;
        assert!(echo.is_empty());
    }
    Ok(())
}
