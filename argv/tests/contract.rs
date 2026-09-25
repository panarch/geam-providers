// Compile the original Hex package from the independently managed embedding fixture.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::embedding::{HostedModuleBuilder, HostedProject};
use geam::{HostComponentProfile, HostProviderConfiguration};

fn project() -> HostedProject<geam_bindings::Profile> {
    HostedProject::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
        geam_bindings::ROOT_MODULE,
        geam_bindings::host_providers,
    )
}

#[test]
fn original_argv_load_uses_the_host_snapshot_and_repeats_it() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = project().compile().expect("original Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("program plans"))
            .expect("source signatures bind");
    let mut module = bindings.seal().expect("module seals");
    let mut state = geam_bindings::RunStateInputs {
        argv: HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    *<geam_bindings::Profile as HostComponentProfile<geam_argv::Component>>::component_state(
        &mut state,
    ) = geam_argv::RunState::new(
        "host-runtime".into(),
        "gleam-program".into(),
        vec!["--flag".into(), "한글".into()],
    );
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(
                            &functions.verify,
                            (
                                "host-runtime".into(),
                                "gleam-program".into(),
                                "--flag".into(),
                                "한글".into(),
                            ),
                        )
                        .await
                        .expect("original argv.load with arguments")
                );
            }),
        )
        .expect("source execution succeeds");
}

#[test]
fn original_argv_load_accepts_an_empty_argument_list() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = project().compile().expect("original Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("program plans"))
            .expect("source signatures bind");
    let mut module = bindings.seal().expect("module seals");
    let mut state = geam_bindings::RunStateInputs {
        argv: HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    *<geam_bindings::Profile as HostComponentProfile<geam_argv::Component>>::component_state(
        &mut state,
    ) = geam_argv::RunState::new("host-runtime".into(), "gleam-program".into(), vec![]);
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(
                            &functions.verify_empty,
                            ("host-runtime".into(), "gleam-program".into()),
                        )
                        .await
                        .expect("original argv.load without arguments")
                );
            }),
        )
        .expect("source execution succeeds");
}
