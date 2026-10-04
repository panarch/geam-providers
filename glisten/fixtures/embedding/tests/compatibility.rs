//! Both live and freshly generated prepared bindings must communicate and stop.
#[test]
fn original_tcp_user_selector_and_tls_servers_run_in_live_and_prepared_embedding() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let status = std::process::Command::new("python3")
        .arg(root.join("../run.py"))
        .arg(env!("CARGO_BIN_EXE_geam-glisten-embedding"))
        .status()
        .unwrap();
    assert!(status.success());
}
