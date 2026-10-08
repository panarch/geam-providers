//! Tests that enter the unchanged Hex package through the public Geam boundary.

#[test]
fn unchanged_package_runs_through_the_public_boundary_and_prepares() {
    use crate::test_support::{Profile, execution_host::TestHost, providers, state};
    let root = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/gleam");
    let typed =
        geam::compile_typed_host_project::<Profile>(&root, "geam_gramps_fixture", providers([]))
            .unwrap();
    let mut state = state(typed.package_resources().clone());
    let plan = geam::plan_host_program(typed).unwrap();
    let mut execution = geam::HostedExecution::try_from_module_plan(plan).unwrap();
    let host = TestHost::default();
    assert_eq!(
        host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
            .unwrap()
            .try_into_value()
            .unwrap(),
        geam::Value::Bool(true)
    );
    let typed =
        geam::compile_typed_host_project::<Profile>(&root, "geam_gramps_fixture", providers([]))
            .unwrap();
    geam::PreparedHostedEntry::try_from_module_plan(geam::plan_host_program(typed).unwrap())
        .unwrap();
}

#[test]
fn downloaded_source_matches_the_native_contract_checksum() {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/gleam/build/packages/gramps");
    let entries = include_str!("../fixtures/upstream.sha256")
        .lines()
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 4);
    for entry in entries {
        let (expected, path) = entry.split_once("  ").unwrap();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(root.join(path)).unwrap())
            ),
            expected,
            "{path}"
        );
    }
}
