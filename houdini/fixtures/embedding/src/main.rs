mod geam_bindings;

use geam::HostProviderConfiguration;
use geam::embedding::HostedModuleBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project().compile()?;
    let builder = HostedModuleBuilder::new(program)?;
    let (bindings, functions) = geam_bindings::bind(builder)?;
    let mut module = bindings.seal()?;
    let mut state = geam_bindings::RunStateInputs {
        houdini: HostProviderConfiguration::empty(),
    }
    .initialize()?;
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
                let escaped = scope
                    .call(&functions.escape, ("&한글<".into(),))
                    .await
                    .expect("public escape call");
                assert_eq!(escaped.as_str(), Ok("&amp;한글&lt;"));
                let raw = scope
                    .call(
                        &functions.escape,
                        (geam::StringValue::from_bytes(vec![255, b'<', 0, b'&', 128]),),
                    )
                    .await
                    .expect("original escape preserves non-HTML bytes");
                assert_eq!(raw.as_bytes(), b"\xff&lt;\x00&amp;\x80");
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))?;
    Ok(())
}
