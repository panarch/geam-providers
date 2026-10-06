#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("embedded");
    let root = root.to_str().ok_or("temporary path is not Unicode")?;
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let builder = HostedModuleBuilder::new(program)?;
    let (bindings, functions) = geam_bindings::bind(builder)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        filepath: HostProviderConfiguration::empty(),
        simplifile: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                let failure = scope
                    .call(
                        &functions.resolve_path,
                        (geam::StringValue::from_bytes(vec![255]),),
                    )
                    .await
                    .expect_err("resolve requires a UTF-8 path");
                assert!(failure.to_string().contains("path is not UTF-8"));
                let (path, expected) = if cfg!(windows) {
                    // filepath 1.1.2 joins the drive's trailing "/" with "/".
                    (r"C:\geam\fixtures\..\résumé.txt", "c://geam/résumé.txt")
                } else {
                    ("/geam/fixtures/../résumé.txt", "/geam/résumé.txt")
                };
                let resolved = scope
                    .call(&functions.resolve_path, (path.into(),))
                    .await
                    .expect("valid resolve after a host failure");
                assert_eq!(resolved.as_bytes(), expected.as_bytes());
                assert!(
                    scope
                        .call(&functions.verify, (root.into(),))
                        .await
                        .expect("original simplifile contract")
                );
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))?;
    Ok(())
}
