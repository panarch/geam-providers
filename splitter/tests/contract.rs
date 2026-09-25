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
        geam_bindings::host_providers::<Vec<geam::gleam_stdlib::IoOutput>>,
    );
    let program = project.compile().expect("original Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("host program plans"))
            .expect("generated source signatures bind");
    let mut module = bindings.seal().expect("host module seals");
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        splitter: geam::HostProviderConfiguration::empty(),
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
                let parts = scope
                    .call(&functions.split_commas, ("a,,é,".into(),))
                    .await
                    .expect("source call");
                assert_eq!(
                    (0..parts.len())
                        .map(|index| parts.read_item(index, Clone::clone).expect("list item"))
                        .collect::<Vec<_>>(),
                    vec![
                        geam::StringValue::from("a"),
                        "".into(),
                        "é".into(),
                        "".into(),
                    ]
                );
            }),
        )
        .expect("host execution completes");
}
