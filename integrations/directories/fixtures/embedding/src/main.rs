#[allow(dead_code)]
mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("embedded");
    let root = root.to_str().ok_or("temporary path is not Unicode")?;
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let initial = HostProviderConfiguration::new(BTreeMap::from([(
        "GEAM_DIRECTORIES_SEED".into(),
        "ready".into(),
    )]));
    let configuration =
        HostProviderConfiguration::new(BTreeMap::from([("values".into(), initial.into())]));
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        envoy: configuration,
        filepath: HostProviderConfiguration::empty(),
        platform: HostProviderConfiguration::empty(),
        simplifile: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();

    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            assert!(scope.call(&functions.seed_is, ("ready".into(),)).await?);
            assert!(scope.call(&functions.verify, (root.into(),)).await?);
            Ok::<(), Box<dyn std::error::Error>>(())
        }),
    )??;
    executor.block_on(
        module.with_execution(&host, &mut state, &mut echo, async |scope| {
            assert!(scope.call(&functions.seed_is, ("ready".into(),)).await?);
            Ok::<(), Box<dyn std::error::Error>>(())
        }),
    )??;

    let fresh_root = temp.path().join("fresh");
    let fresh_root = fresh_root.to_str().ok_or("temporary path is not Unicode")?;
    let mut fresh = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        envoy: HostProviderConfiguration::new(BTreeMap::from([(
            "values".into(),
            HostProviderConfiguration::empty().into(),
        )])),
        filepath: HostProviderConfiguration::empty(),
        platform: HostProviderConfiguration::empty(),
        simplifile: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    executor.block_on(
        module.with_execution(&host, &mut fresh, &mut echo, async |scope| {
            assert!(!scope.call(&functions.seed_is, ("ready".into(),)).await?);
            assert!(scope.call(&functions.verify, (fresh_root.into(),)).await?);
            Ok::<(), Box<dyn std::error::Error>>(())
        }),
    )??;
    assert!(echo.is_empty());
    Ok(())
}
