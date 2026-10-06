mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project().compile()?;
    let builder = HostedModuleBuilder::new(program)?;
    let (bindings, functions) = geam_bindings::bind(builder)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        gleam_regexp: HostProviderConfiguration::empty(),
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
                let decorated = scope
                    .call(&functions.decorate, (r"\w+".into(), "hi, joe".into()))
                    .await
                    .expect("decorated result");
                assert_eq!(decorated, Ok("[hi], [joe]".into()));
                let failure = scope
                    .call(&functions.callback_failure, ())
                    .await
                    .expect_err("callback panic reaches embedding caller");
                assert!(failure.to_string().contains("regexp callback failed"));
                let invalid = geam::StringValue::from_bytes(vec![255]);
                let failures = [
                    scope
                        .call(&functions.check_input, (invalid.clone(),))
                        .await
                        .expect_err("check requires text"),
                    scope
                        .call(&functions.split_input, (invalid.clone(),))
                        .await
                        .err()
                        .expect("split requires text"),
                    scope
                        .call(&functions.scan_input, (invalid.clone(),))
                        .await
                        .err()
                        .expect("scan requires text"),
                    scope
                        .call(&functions.replace_input, ("a".into(), invalid.clone()))
                        .await
                        .expect_err("replacement syntax requires text"),
                    scope
                        .call(&functions.map_input, (invalid.clone(), "x".into()))
                        .await
                        .expect_err("callback matching requires text"),
                ];
                for failure in failures {
                    assert!(
                        failure
                            .to_string()
                            .contains("regular expression text is not UTF-8")
                    );
                }
                let raw = scope
                    .call(&functions.map_input, ("ba!a?".into(), invalid))
                    .await
                    .expect("callback replacement preserves bytes");
                assert_eq!(raw.as_bytes(), &[b'b', 255, b'!', 255, b'?']);
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))?;
    Ok(())
}
