#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        global_value: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    for initial_value in [41, 99] {
        executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    assert_eq!(
                        scope
                            .call(&functions.first, (initial_value.into(),))
                            .await
                            .expect("first"),
                        initial_value.into(),
                    );
                    assert_eq!(
                        scope.call(&functions.second, ()).await.expect("second"),
                        initial_value.into(),
                    );
                    assert_eq!(
                        scope.call(&functions.other, ()).await.expect("other"),
                        7.into()
                    );
                    let first = geam::StringValue::from_bytes(vec![b'n', 255]);
                    let second = geam::StringValue::from_bytes(vec![b'n', 254]);
                    assert_eq!(
                        scope
                            .call(&functions.named, (first.clone(), initial_value.into()))
                            .await
                            .expect("first raw name"),
                        initial_value.into()
                    );
                    assert_eq!(
                        scope
                            .call(&functions.named, (first, 0.into()))
                            .await
                            .expect("equal raw name reuses value"),
                        initial_value.into()
                    );
                    assert_eq!(
                        scope
                            .call(&functions.named, (second, 7.into()))
                            .await
                            .expect("distinct raw name"),
                        7.into()
                    );
                }),
            )?
            .try_into_value()
            .map_err(|status| format!("unexpected application exit {status}"))?;
    }
    Ok(())
}
