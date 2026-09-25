#[allow(dead_code)]
mod geam_bindings;

use geam::embedding::HostedModuleBuilder;
use geam::{HostComponentProfile, HostProviderConfiguration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        argv: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    *<geam_bindings::Profile as HostComponentProfile<geam_argv::Component>>::component_state(
        &mut state,
    ) = geam_argv::RunState::new(
        "/host/runtime".into(),
        "/host/project".into(),
        vec!["--flag".into(), "key=value".into()],
    );
    let mut echo = Vec::new();

    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            assert!(
                scope
                    .call(
                        &functions.verify,
                        (
                            "/host/runtime".into(),
                            "/host/project".into(),
                            "--flag".into(),
                            "key=value".into(),
                        ),
                    )
                    .await
                    .expect("original argv contract")
            );
        }),
    )?;
    Ok(())
}
