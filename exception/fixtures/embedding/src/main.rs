#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        exception: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(&functions.verify, ())
                        .await
                        .expect("source contract")
                );
                assert!(
                    scope
                        .call(&functions.cleanup_failure_wins, ())
                        .await
                        .expect("cleanup failure contract")
                );
                assert!(
                    scope
                        .call(&functions.native_failure_is_caught, ())
                        .await
                        .expect("native failure contract")
                );
                assert!(
                    scope
                        .call(&functions.nested_cleanup_order, ())
                        .await
                        .expect("nested cleanup contract")
                );
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))?;
    assert_eq!(
        state
            .stdlib()
            .io_outputs()
            .iter()
            .map(|output| output.text().as_str())
            .collect::<Vec<_>>(),
        [
            "cleanup success\n",
            "crash cleanup\n",
            "cleanup failure\n",
            "success inner\n",
            "success outer\n",
            "failure inner\n",
            "failure outer\n",
        ]
    );
    Ok(())
}
