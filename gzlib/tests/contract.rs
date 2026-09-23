// The generated project() uses the embedding crate's manifest path; this test
// opens that same original Gleam project from the provider crate's manifest path.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::HostProviderComponentRegistration;
use geam::embedding::{HostedModuleBuilder, HostedProject};
use std::process::Command;

#[test]
fn registration_matches_the_five_original_externals() {
    let providers = <geam_gzlib::Component as HostProviderComponentRegistration<
        geam_bindings::Profile,
    >>::providers()
    .expect("static gzlib registration");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].package(), "gzlib");
    assert_eq!(providers[0].module(), "gzlib");
    let names: Vec<_> = providers[0]
        .functions()
        .map(|schema| schema.name().as_str())
        .collect();
    assert_eq!(
        names,
        [
            "compress",
            "compress_custom",
            "uncompress",
            "crc32",
            "crc32_continue",
        ]
    );

    geam_bindings::RunStateInputs {
        gzlib: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("stateless provider initializes");
}

#[test]
fn original_gleam_contract_and_erlang_zlib_interoperability() {
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
        gzlib: geam::HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    let mut echo = Vec::new();

    let compressed = executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(
                    scope
                        .call(&functions.verify, ())
                        .await
                        .expect("source contract")
                );
                scope
                    .call(&functions.compressed_sample, ())
                    .await
                    .expect("public compress call")
            }),
        )
        .expect("host execution completes");
    assert!(echo.is_empty());

    let bytes = compressed
        .bytes()
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let script =
        format!("case zlib:uncompress(<<{bytes}>>) of <<97,98,99>> -> halt(0); _ -> halt(1) end.");
    let output = Command::new("erl")
        .args(["-noshell", "-eval", &script])
        .output()
        .expect("Erlang is available for the original zlib backend");
    assert!(
        output.status.success(),
        "Erlang could not decode provider output: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
