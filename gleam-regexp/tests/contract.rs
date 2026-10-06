// The generated project() uses the embedding crate's manifest path; this test
// opens that same Gleam project from the provider crate's manifest path.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::embedding::{HostedModuleBuilder, HostedProject};

#[test]
fn original_gleam_package_runs_through_public_host_boundary() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("single-threaded test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let project = HostedProject::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
        geam_bindings::ROOT_MODULE,
        geam_bindings::host_providers,
    );
    let program = project.compile().expect("original Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("host program plans"))
            .expect("generated source signatures bind");
    let mut module = bindings.seal().expect("host module seals");
    let mut state = geam_bindings::RunStateInputs {
        gleam_regexp: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
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
                assert_eq!(
                    scope
                        .call(&functions.decorate, (r"\w+".into(), "hi, joe".into()))
                        .await
                        .expect("source call"),
                    Ok("[hi], [joe]".into()),
                );
                let failure = scope
                    .call(&functions.callback_failure, ())
                    .await
                    .expect_err("callback panic must reach the caller");
                assert!(failure.to_string().contains("regexp callback failed"));
                let invalid = geam::StringValue::from_bytes(vec![b'a', 255]);
                assert_eq!(
                    scope
                        .call(&functions.decorate, (invalid.clone(), "a".into()))
                        .await
                        .expect("compile error remains a Gleam Result"),
                    Err("invalid utf-8 sequence of 1 bytes from index 1".into())
                );
                let failure = scope
                    .call(&functions.check_input, (invalid.clone(),))
                    .await
                    .expect_err("check requires text");
                assert!(
                    failure
                        .to_string()
                        .contains("regular expression text is not UTF-8")
                );
                let failure = scope
                    .call(&functions.split_input, (invalid.clone(),))
                    .await
                    .err()
                    .expect("split requires text");
                assert!(
                    failure
                        .to_string()
                        .contains("regular expression text is not UTF-8")
                );
                let failure = scope
                    .call(&functions.scan_input, (invalid.clone(),))
                    .await
                    .err()
                    .expect("scan requires text");
                assert!(
                    failure
                        .to_string()
                        .contains("regular expression text is not UTF-8")
                );
                let failure = scope
                    .call(&functions.replace_input, (invalid.clone(), "x".into()))
                    .await
                    .expect_err("replacement input requires text");
                assert!(
                    failure
                        .to_string()
                        .contains("regular expression text is not UTF-8")
                );
                let failure = scope
                    .call(&functions.replace_input, ("a".into(), invalid.clone()))
                    .await
                    .expect_err("replacement syntax requires text");
                assert!(
                    failure
                        .to_string()
                        .contains("regular expression text is not UTF-8")
                );
                let failure = scope
                    .call(&functions.map_input, (invalid, "x".into()))
                    .await
                    .expect_err("callback matching requires text");
                assert!(
                    failure
                        .to_string()
                        .contains("regular expression text is not UTF-8")
                );
                let substitute = geam::StringValue::from_bytes(vec![0, 255]);
                let result = scope
                    .call(&functions.map_input, ("ba!a?".into(), substitute.clone()))
                    .await
                    .expect("callback replacement preserves bytes");
                assert_eq!(result.as_bytes(), &[b'b', 0, 255, b'!', 0, 255, b'?']);
                let result = scope
                    .call(&functions.map_input, ("---".into(), substitute))
                    .await
                    .expect("unmatched callback returns its input");
                assert_eq!(result, "---");
            }),
        )
        .expect("host execution completes")
        .try_into_value()
        .expect("source execution returns normally");
    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(&functions.verify, ())
                        .await
                        .expect("fresh execution after text errors")
                );
            }),
        )
        .expect("fresh execution completes")
        .try_into_value()
        .expect("normal return");
}
