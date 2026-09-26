#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let initial =
        HostProviderConfiguration::new(BTreeMap::from([("INITIAL".into(), "ready".into())]));
    let configuration =
        HostProviderConfiguration::new(BTreeMap::from([("values".into(), initial.into())]));
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        envoy: configuration,
    }
    .initialize()?;
    let mut echo = Vec::new();

    for _ in 0..2 {
        executor.block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(&functions.verify, ())
                        .await
                        .expect("original envoy contract")
                );
            }),
        )?;
    }
    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            scope
                .call(&functions.write, ("PERSIST".into(), "visible".into()))
                .await
                .expect("write provider state");
        }),
    )?;
    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            assert!(
                scope
                    .call(&functions.contains, ("PERSIST".into(), "visible".into()))
                    .await
                    .expect("read provider state across executions")
            );
            assert!(
                scope
                    .call(&functions.clear, ("PERSIST".into(),))
                    .await
                    .expect("clear provider state")
            );
        }),
    )?;
    assert!(echo.is_empty());
    Ok(())
}
