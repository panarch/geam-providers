// The generated project() uses the embedding crate's manifest path; this test
// opens that same Gleam project from the provider crate's manifest path.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::embedding::{HostedModuleBuilder, HostedProject};
use std::process::Command;

#[test]
fn original_gleam_name_matches_geam_hosted_name() {
    let original = Command::new("gleam")
        .arg("run")
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/gleam"))
        .output()
        .expect("run the original Hex package through Erlang");
    assert!(
        original.status.success(),
        "original Gleam fixture failed: {}",
        String::from_utf8_lossy(&original.stderr)
    );
    let original_name = String::from_utf8(original.stdout).expect("original name is UTF-8");
    let original_name = original_name.trim_end_matches(['\r', '\n']);

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
        operating_system: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                let actual = scope
                    .call(&functions.name, ())
                    .await
                    .expect("original operating_system.name contract");
                assert_eq!(actual.as_str(), original_name);
            }),
        )
        .expect("host execution completes");
}
