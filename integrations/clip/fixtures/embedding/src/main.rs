#[allow(dead_code)]
mod geam_bindings;

use geam::embedding::{HostedModuleBuilder, SharedList, StringValue};
use geam::execution::{ExecutionOutcome, ExitStatus};
use geam::gleam_stdlib::{IoOutput, IoStream};
use geam::{HostComponentProfile, HostProviderConfiguration};

type State = geam_bindings::RunState<Vec<IoOutput>>;
type Profile = geam_bindings::Profile<Vec<IoOutput>>;
const CALLBACK_TRACE: [&str; 9] = [
    "arg.map",
    "arg.try_map",
    "arg.map",
    "arg.try_map",
    "opt.map",
    "opt.try_map",
    "apply",
    "arg.fail",
    "opt.fail",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    verify()?;
    println!(
        "clip embedding: contracts, arguments, results, exit status, IO and state isolation passed"
    );
    Ok(())
}

fn verify() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let alpha = temp.path().join("alpha 한글.txt");
    let beta = temp.path().join("notes space.txt");
    let missing = temp.path().join("missing.txt");
    let fresh_file = temp.path().join("fresh.txt");
    std::fs::write(&alpha, "Rust tools\nquiet line\nRUST async\n한글 Rust\n")?;
    std::fs::write(&beta, "rust lower\r\nRust again  \r\n")?;
    std::fs::write(&fresh_file, "Rust fresh\nrust lower\n")?;
    let alpha = alpha.to_str().ok_or("temporary path is not Unicode")?;
    let beta = beta.to_str().ok_or("temporary path is not Unicode")?;
    let missing = missing.to_str().ok_or("temporary path is not Unicode")?;
    let fresh_file = fresh_file.to_str().ok_or("temporary path is not Unicode")?;
    let executor = tokio::runtime::Builder::new_current_thread().build()?;
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = geam_bindings::project::<Vec<geam::gleam_stdlib::IoOutput>>().compile()?;
    let (bindings, functions) = geam_bindings::bind(HostedModuleBuilder::new(program)?)?;
    let mut module = bindings.seal()?;
    let snapshot = ["", "space value", "--", "-dash", "한글"];
    let mut state = new_state("embedded runtime", "clip-search host", &snapshot)?;
    let mut echo = Vec::new();
    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                assert!(scope.call(&functions.verify_contracts, ()).await?);
                assert_eq!(
                    strings(&scope.call(&functions.arguments, ()).await?),
                    snapshot
                );
                let identity = scope.call(&functions.identity, ()).await?;
                assert_eq!(
                    (
                        identity.0.as_str().expect("Unicode argv identity"),
                        identity.1.as_str().expect("Unicode argv identity")
                    ),
                    ("embedded runtime", "clip-search host")
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))??;
    let callback_trace: Vec<String> = CALLBACK_TRACE.iter().map(|line| (*line).into()).collect();
    assert_outputs(&mut state, IoStream::Stdout, &callback_trace);

    set_arguments(
        &mut state,
        "embedded runtime",
        "clip-search host",
        &["search", "-p", "Rust", "-i", alpha, beta],
    );
    let expected = vec![
        format!("{alpha}:1:Rust tools"),
        format!("{alpha}:3:RUST async"),
        format!("{alpha}:4:한글 Rust"),
        format!("{beta}:1:rust lower"),
        format!("{beta}:2:Rust again  "),
    ];
    // Repeat the same parser/application in the same state, including its callbacks.
    for _ in 0..2 {
        executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    assert!(scope.call(&functions.verify_contracts, ()).await?);
                    let outcome = scope.call(&functions.run, ()).await?;
                    let lines = scope
                        .call(&functions.completed, (outcome.clone(),))
                        .await?
                        .expect("Completed");
                    assert_eq!(strings(&lines), expected);
                    assert!(
                        scope
                            .call(&functions.help_text, (outcome.clone(),))
                            .await?
                            .is_err()
                    );
                    assert!(
                        scope
                            .call(&functions.failure_message, (outcome,))
                            .await?
                            .is_err()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                }),
            )?
            .try_into_value()
            .map_err(|status| format!("unexpected application exit {status}"))??;
        let effects: Vec<_> = callback_trace.iter().chain(&expected).cloned().collect();
        assert_outputs(&mut state, IoStream::Stdout, &effects);
    }

    let mut fresh = new_state(
        "fresh runtime",
        "fresh program",
        &["count", "--pattern", "Rust", fresh_file],
    )?;
    executor
        .block_on(
            module.with_execution(&host, &mut fresh, &mut echo, async |scope| {
                let identity = scope.call(&functions.identity, ()).await?;
                assert_eq!(
                    (
                        identity.0.as_str().expect("Unicode argv identity"),
                        identity.1.as_str().expect("Unicode argv identity")
                    ),
                    ("fresh runtime", "fresh program")
                );
                assert_eq!(
                    strings(&scope.call(&functions.arguments, ()).await?),
                    ["count", "--pattern", "Rust", fresh_file]
                );
                let outcome = scope.call(&functions.run, ()).await?;
                assert_eq!(
                    strings(
                        &scope
                            .call(&functions.completed, (outcome,))
                            .await?
                            .expect("Completed")
                    ),
                    [format!("{fresh_file}:1")]
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))??;
    assert_outputs(&mut fresh, IoStream::Stdout, &[format!("{fresh_file}:1")]);
    assert!(state.stdlib().io_outputs().is_empty());

    // The first file is readable; a later failure must not print partial results.
    set_arguments(
        &mut state,
        "embedded runtime",
        "clip-search host",
        &["search", "-p", "Rust", alpha, missing],
    );
    let read_error = format!("cannot read {missing}: No such file or directory");
    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                let outcome = scope.call(&functions.run, ()).await?;
                assert!(
                    scope
                        .call(&functions.completed, (outcome.clone(),))
                        .await?
                        .is_err()
                );
                assert_eq!(
                    scope
                        .call(&functions.missing_file, (outcome.clone(),))
                        .await?
                        .expect("ReadFailure(Enoent)"),
                    missing
                );
                assert_eq!(
                    scope
                        .call(&functions.failure_message, (outcome,))
                        .await?
                        .expect("Failed"),
                    read_error.as_str()
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))??;
    assert_outputs(&mut state, IoStream::Stderr, &[read_error]);
    assert!(fresh.stdlib().io_outputs().is_empty());

    set_arguments(
        &mut state,
        "embedded runtime",
        "clip-search host",
        &["search", "-p", "(", missing],
    );
    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                let outcome = scope.call(&functions.run, ()).await?;
                assert!(
                    scope
                        .call(&functions.invalid_pattern, (outcome.clone(),))
                        .await?
                );
                assert!(
                    scope
                        .call(&functions.missing_file, (outcome.clone(),))
                        .await?
                        .is_err()
                );
                assert_eq!(
                    scope
                        .call(&functions.failure_message, (outcome,))
                        .await?
                        .expect("Failed"),
                    "invalid regular expression"
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))??;
    assert_outputs(
        &mut state,
        IoStream::Stderr,
        &["invalid regular expression".into()],
    );

    // A successful call after errors reads current file contents, not cached output.
    std::fs::write(alpha, "Rust changed\n")?;
    set_arguments(
        &mut state,
        "embedded runtime",
        "clip-search host",
        &["search", "-p", "Rust", alpha],
    );
    let changed = format!("{alpha}:1:Rust changed");
    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                let outcome = scope.call(&functions.run, ()).await?;
                assert_eq!(
                    strings(
                        &scope
                            .call(&functions.completed, (outcome,))
                            .await?
                            .expect("Completed")
                    ),
                    std::slice::from_ref(&changed)
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))??;
    assert_outputs(&mut state, IoStream::Stdout, &[changed]);

    // Reuse the independent state after the other state failed and changed.
    executor
        .block_on(
            module.with_execution(&host, &mut fresh, &mut echo, async |scope| {
                let outcome = scope.call(&functions.run, ()).await?;
                assert_eq!(
                    strings(
                        &scope
                            .call(&functions.completed, (outcome,))
                            .await?
                            .expect("Completed")
                    ),
                    [format!("{fresh_file}:1")]
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))??;
    assert_outputs(&mut fresh, IoStream::Stdout, &[format!("{fresh_file}:1")]);

    let help = "Usage: clip-search <search|count> --pattern PATTERN [-i] FILE...\n\nCommands:\n  search  Print matching lines\n  count   Print matching line counts\n\nUse --help after a command for its options.";
    set_arguments(&mut fresh, "fresh runtime", "fresh program", &["--help"]);
    executor
        .block_on(
            module.with_execution(&host, &mut fresh, &mut echo, async |scope| {
                let outcome = scope.call(&functions.run, ()).await?;
                assert!(
                    scope
                        .call(&functions.completed, (outcome.clone(),))
                        .await?
                        .is_err()
                );
                assert!(
                    scope
                        .call(&functions.failure_message, (outcome.clone(),))
                        .await?
                        .is_err()
                );
                assert_eq!(
                    scope
                        .call(&functions.help_text, (outcome,))
                        .await?
                        .expect("Help"),
                    help
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }),
        )?
        .try_into_value()
        .map_err(|status| format!("unexpected application exit {status}"))??;
    assert_outputs(&mut fresh, IoStream::Stdout, &[help.into()]);

    // main terminates its execution scope without terminating the Rust host.
    for (arguments, status, message) in [
        (
            vec!["search", "-p", "Rust", alpha, missing],
            1,
            format!("cannot read {missing}: No such file or directory"),
        ),
        (
            vec!["search", "-p", "(", missing],
            2,
            "invalid regular expression".into(),
        ),
        (vec![], 2, "arguments: No subcommand provided".into()),
    ] {
        set_arguments(
            &mut state,
            "embedded runtime",
            "clip-search host",
            &arguments,
        );
        let mut continued = false;
        let outcome = executor.block_on(module.with_execution(
            &host,
            &mut state,
            &mut echo,
            async |scope| {
                scope.call(&functions.main, ()).await?;
                continued = true;
                Ok::<(), Box<dyn std::error::Error>>(())
            },
        ))?;
        assert!(
            matches!(outcome, ExecutionOutcome::Exited(code) if code == ExitStatus::new(status))
        );
        assert!(!continued, "application exit prevents scope continuation");
        assert_outputs(&mut state, IoStream::Stderr, &[message]);
        assert!(fresh.stdlib().io_outputs().is_empty());
    }

    // A fresh scope reuses the same host, sealed module and state after exits.
    set_arguments(
        &mut state,
        "embedded runtime",
        "clip-search host",
        &["search", "-p", "Rust", alpha],
    );
    let outcome = executor.block_on(module.with_execution(
        &host,
        &mut state,
        &mut echo,
        async |scope| scope.call(&functions.main, ()).await,
    ))?;
    assert!(matches!(outcome, ExecutionOutcome::Returned(Ok(()))));
    assert_outputs(
        &mut state,
        IoStream::Stdout,
        &[format!("{alpha}:1:Rust changed")],
    );

    // The independent state and help path also remain available.
    let outcome = executor.block_on(module.with_execution(
        &host,
        &mut fresh,
        &mut echo,
        async |scope| scope.call(&functions.main, ()).await,
    ))?;
    assert!(matches!(outcome, ExecutionOutcome::Returned(Ok(()))));
    assert_outputs(&mut fresh, IoStream::Stdout, &[help.into()]);
    assert!(state.stdlib().io_outputs().is_empty());
    assert!(fresh.stdlib().io_outputs().is_empty());
    assert!(echo.is_empty());
    Ok(())
}

fn new_state(
    runtime: &str,
    program: &str,
    arguments: &[&str],
) -> Result<State, Box<dyn std::error::Error>> {
    let mut state = geam_bindings::RunStateInputs {
        stdlib: geam::gleam_stdlib::GleamStdlibRunState::from_seed([0; 32]),
        argv: HostProviderConfiguration::empty(),
        filepath: HostProviderConfiguration::empty(),
        geam_clip_search: HostProviderConfiguration::empty(),
        gleam_regexp: HostProviderConfiguration::empty(),
        simplifile: HostProviderConfiguration::empty(),
    }
    .initialize()?;
    set_arguments(&mut state, runtime, program, arguments);
    Ok(state)
}

fn set_arguments(state: &mut State, runtime: &str, program: &str, arguments: &[&str]) {
    *<Profile as HostComponentProfile<geam_argv::Component>>::component_state(state) =
        geam_argv::RunState::new(
            runtime.into(),
            program.into(),
            arguments.iter().map(|value| (*value).into()).collect(),
        );
}

fn strings(values: &SharedList<StringValue>) -> Vec<String> {
    (0..values.len())
        .map(|index| {
            values
                .read_item(index, |value| {
                    value.as_str().expect("Unicode search result").to_owned()
                })
                .expect("in-bounds source list item")
        })
        .collect()
}

fn assert_outputs(state: &mut State, stream: IoStream, lines: &[String]) {
    let outputs = state.stdlib_mut().take_io_outputs();
    let actual: Vec<_> = outputs
        .iter()
        .map(|output| {
            (
                output.stream(),
                output.text().as_str().expect("Unicode fixture output"),
            )
        })
        .collect();
    let text: Vec<_> = lines.iter().map(|line| format!("{line}\n")).collect();
    let expected: Vec<_> = text.iter().map(|line| (stream, line.as_str())).collect();
    assert_eq!(actual, expected);
    assert!(state.stdlib().io_outputs().is_empty());
}

#[cfg(test)]
mod tests {
    #[test]
    fn public_embedding_preserves_contracts_arguments_results_and_state() {
        super::verify().expect("clip integration should pass through public embedding");
    }
}
