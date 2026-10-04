//! Pinned, unchanged upstream source linked with the complete native signatures.
use crate::test_support::{execution_fixture, project};
use geam::Value;
use sha2::{Digest, Sha256};

#[test]
fn original_source_matches_the_pinned_contract() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/gleam/build/packages/glisten");
    let entries = include_str!("../fixtures/upstream.sha256")
        .lines()
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 9);
    for entry in entries {
        let (expected, path) = entry.split_once("  ").unwrap();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root.join(path)).unwrap())
            ),
            expected,
            "upstream {path}"
        );
    }
}

#[test]
fn original_package_and_native_signatures_plan_and_execute() {
    let (mut execution, mut state) = project("glisten_source_preflight");
    let host = execution_fixture::TestHost::default();
    assert_eq!(
        host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
            .map(|outcome| outcome
                .try_into_value()
                .expect("fixture must return normally"))
            .unwrap(),
        Value::Nil
    );
}

#[test]
fn original_bind_can_be_prepared() {
    let typed = crate::test_support::typed_project("glisten_source_preflight");
    let prepared =
        geam::PreparedHostedEntry::try_from_module_plan(geam::plan_host_program(typed).unwrap());
    prepared.unwrap();
}
