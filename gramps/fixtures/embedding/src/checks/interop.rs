use crate::geam_bindings;
use geam::embedding::{BitArrayValue, HostedModule};
use std::path::Path;
use std::process::Command;

/// Both compression directions enter the unchanged Gleam package. The independent
/// Erlang peer reads our wire bytes and creates its own stream for our inflater.
pub(super) fn check(
    module: &mut HostedModule<geam_bindings::Profile<Vec<geam::gleam_stdlib::IoOutput>>>,
    functions: &geam_bindings::Functions,
) -> Result<(), Box<dyn std::error::Error>> {
    let messages = [
        vec![],
        b"hello websocket ".repeat(16000),
        b"hello websocket ".repeat(16000),
        (0..100_000).map(|n| (n % 256) as u8).collect(),
    ];
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        erlang: Default::default(),
        gleam_crypto: geam::HostProviderConfiguration::empty(),
        gramps: geam::HostProviderConfiguration::empty(),
    }
    .initialize()?;
    let mut echo = Vec::new();
    for reset in [false, true] {
        let input: Vec<_> = messages
            .iter()
            .map(|bytes| BitArrayValue::from_bytes(bytes.clone()))
            .collect();
        let wire = executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    scope.call(&functions.compress, (input, reset)).await
                }),
            )?
            .try_into_value()
            .expect("compression returns normally")?;
        // Read retained output after the complete compression execution has closed.
        assert_eq!(wire.len(), messages.len());
        let directory = tempfile::tempdir()?;
        for (index, message) in messages.iter().enumerate() {
            std::fs::write(directory.path().join(format!("{index}.message")), message)?;
            wire.read_item(index, |compressed| {
                std::fs::write(
                    directory.path().join(format!("{index}.provider")),
                    compressed.bytes(),
                )
            })
            .expect("one output per message")?;
        }
        let output = Command::new("escript")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../permessage_deflate.escript"))
            .arg(directory.path())
            .arg(reset.to_string())
            .output()?;
        assert!(
            output.status.success(),
            "independent Erlang peer: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let peer: Vec<_> = (0..messages.len())
            .map(|index| {
                std::fs::read(directory.path().join(format!("{index}.erlang")))
                    .map(BitArrayValue::from_bytes)
            })
            .collect::<Result<_, _>>()?;
        let restored = executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    scope.call(&functions.decompress, (peer, reset)).await
                }),
            )?
            .try_into_value()
            .expect("decompression returns normally")?;
        assert_eq!(restored.len(), messages.len());
        for (index, expected) in messages.iter().enumerate() {
            restored
                .read_item(index, |actual| {
                    assert_eq!(actual.bytes(), expected);
                })
                .expect("one output per message");
            let earlier = std::fs::read(directory.path().join(format!("{index}.provider")))?;
            wire.read_item(index, |earlier_wire| {
                assert_eq!(earlier_wire.bytes(), earlier);
            })
            .expect("retained predecessor output");
        }
    }
    Ok(())
}
