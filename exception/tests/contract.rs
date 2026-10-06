// The generated bindings compile the original Hex package from the embedding
// fixture while this test opens the project from the provider crate root.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::embedding::{HostedModuleBuilder, HostedProject};
use std::process::Command;

#[test]
fn original_exception_source_preserves_results_and_cleanup_order() {
    let original = Command::new("gleam")
        .arg("run")
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/gleam"))
        .output()
        .expect("run the original Hex package on Erlang");
    assert!(
        original.status.success(),
        "original exception fixture failed: {}",
        String::from_utf8_lossy(&original.stderr)
    );
    let original_output = String::from_utf8(original.stdout).expect("original output is UTF-8");
    assert_eq!(
        original_output,
        "cleanup success\ncrash cleanup\ncleanup failure\n"
    );

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
            .expect("original declarations bind");
    let mut module = bindings.seal().expect("host module seals");
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        exception: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(scope.call(&functions.verify, ()).await.expect("common API"));
                assert!(
                    scope
                        .call(&functions.cleanup_failure_wins, ())
                        .await
                        .expect("cleanup failure precedence")
                );
                assert!(
                    scope
                        .call(&functions.native_failure_is_caught, ())
                        .await
                        .expect("native host failure maps to Errored")
                );
                assert!(
                    scope
                        .call(&functions.nested_cleanup_order, ())
                        .await
                        .expect("nested cleanup order")
                );
            }),
        )
        .expect("host execution completes")
        .try_into_value()
        .expect("source execution returns normally");

    let output = state
        .stdlib()
        .io_outputs()
        .iter()
        .map(|event| event.text().as_str().expect("Unicode fixture output"))
        .collect::<Vec<_>>();
    assert_eq!(output[..3].concat(), original_output);
    assert_eq!(
        &output[3..],
        [
            "success inner\n",
            "success outer\n",
            "failure inner\n",
            "failure outer\n",
        ]
    );
}

#[test]
fn callback_cancellation_keeps_its_execution_domain_and_cleanup_boundary() {
    use geam::embedding::{CallError, FunctionDeclaration};
    use geam::{
        HostCall, HostCallContinuation, HostCallError, HostConstructions, HostExecutionError,
        HostProviderComponentRegistration, HostProviderModule, HostProviderSet,
        HostRegistrationError, HostTypeListEnd,
    };
    use std::task::Poll;

    type Profile = geam_bindings::Profile<Vec<geam::gleam_stdlib::IoOutput>>;

    fn cancel<'call>(
        call: HostCall<'call, Profile, geam::FutureComponent, ()>,
        constructions: HostConstructions<'call, HostTypeListEnd>,
    ) -> Result<HostCallContinuation<'call, ()>, HostCallError> {
        Ok(call.resume(constructions, |_| {
            Box::pin(async {
                let mut pending = true;
                std::future::poll_fn(|cx| {
                    if pending {
                        pending = false;
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    } else {
                        Poll::Ready(())
                    }
                })
                .await;
                Err(HostExecutionError::Cancelled)
            })
        }))
    }

    fn providers() -> Result<HostProviderSet<Profile>, HostRegistrationError> {
        let mut providers = <geam::gleam_stdlib::Component as HostProviderComponentRegistration<
            Profile,
        >>::providers()?;
        providers.extend(
            <geam_exception::Component as HostProviderComponentRegistration<Profile>>::providers()?,
        );
        providers.push(
            HostProviderModule::new("geam_exception_embedding", "exception_cancellation")?
                .with_resumable_function::<geam::FutureComponent, (), (), HostTypeListEnd, _>(
                "cancel", cancel,
            )?,
        );
        HostProviderSet::from_providers(providers)
    }

    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let project = HostedProject::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
        "exception_cancellation",
        providers,
    );
    let (mut bindings, rescue) = HostedModuleBuilder::new(project.compile().unwrap())
        .unwrap()
        .function(FunctionDeclaration::<(), ()>::new("rescue_cancel"))
        .unwrap();
    let defer = bindings
        .function(FunctionDeclaration::<(), ()>::new("defer_cancel"))
        .unwrap();
    let on_crash = bindings
        .function(FunctionDeclaration::<(), ()>::new("on_crash_cancel"))
        .unwrap();
    let cleanup_failure = bindings
        .function(FunctionDeclaration::<(), ()>::new(
            "defer_cancel_cleanup_failure",
        ))
        .unwrap();
    let mut module = bindings.seal().unwrap();

    for (function, expected_result, expected_output) in [
        (&rescue, Err(CallError::Cancelled), &[][..]),
        (&defer, Err(CallError::Cancelled), &["defer cleanup\n"][..]),
        (&on_crash, Err(CallError::Cancelled), &[][..]),
        (&cleanup_failure, Ok(()), &[][..]),
    ] {
        let mut state = geam_bindings::RunStateInputs {
            stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
            exception: geam::HostProviderConfiguration::empty(),
        }
        .initialize()
        .unwrap();
        let mut echo = Vec::new();
        let result = executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    scope.call(function, ()).await
                }),
            )
            .unwrap()
            .try_into_value()
            .expect("source execution returns normally");
        assert_eq!(result, expected_result);
        let output = state
            .stdlib()
            .io_outputs()
            .iter()
            .map(|event| event.text().as_str().expect("Unicode fixture output"))
            .collect::<Vec<_>>();
        assert_eq!(output, expected_output);
    }
}
