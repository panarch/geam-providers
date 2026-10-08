mod interop;

use crate::geam_bindings;
use geam::embedding::{HostedModule, HostedModuleBuilder};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let typed = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let (builder, live_functions) = geam_bindings::bind(HostedModuleBuilder::new(typed)?)?;
    let mut live = builder.seal()?;
    check(&mut live, &live_functions)?;
    interop::check(&mut live, &live_functions)?;
    println!("live: package, echo, failure recovery and fresh execution contracts passed");

    let (mut prepared, prepared_functions) =
        geam_bindings::load::<Vec<geam::gleam_stdlib::IoOutput>>()?;
    check(&mut prepared, &prepared_functions)?;
    interop::check(&mut prepared, &prepared_functions)?;
    println!("prepared: package, echo, failure recovery and fresh execution contracts passed");
    Ok(())
}

fn check(
    module: &mut HostedModule<geam_bindings::Profile<Vec<geam::gleam_stdlib::IoOutput>>>,
    functions: &geam_bindings::Functions,
) -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: Default::default(),
        gleam_crypto: geam::HostProviderConfiguration::empty(),
        gramps: geam::HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();
    for _ in 0..2 {
        executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    assert!(
                        scope
                            .call(&functions.verify, ())
                            .await
                            .expect("unchanged package contracts")
                    );
                    assert_eq!(
                        scope
                            .call(&functions.round_trip, ("hello websocket".into(),))
                            .await
                            .expect("shared echo session")
                            .bytes(),
                        b"hello websocket"
                    );
                    assert!(
                        scope
                            .call(&functions.closed, ())
                            .await
                            .expect_err("closed context")
                            .to_string()
                            .contains("compression context is closed")
                    );
                    assert!(
                        scope
                            .call(&functions.malformed, ())
                            .await
                            .expect_err("malformed stream")
                            .to_string()
                            .contains("raw DEFLATE backend failed")
                    );
                    assert!(
                        scope
                            .call(&functions.partial, ())
                            .await
                            .expect_err("partial byte")
                            .to_string()
                            .contains("compression input must contain complete bytes")
                    );
                    assert!(
                        scope
                            .call(&functions.verify, ())
                            .await
                            .expect("recovery after failed calls")
                    );
                    let key = scope
                        .call(&functions.client_key, ())
                        .await
                        .expect("OS entropy through real crypto provider");
                    assert_eq!(key.as_bytes().len(), 24);
                }),
            )?
            .try_into_value()
            .expect("normal source completion");
    }
    Ok(())
}
