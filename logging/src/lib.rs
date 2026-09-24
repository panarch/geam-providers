//! Native externals for the unmodified `logging` 1.5.0 package.

use geam::provider::{Configuration, HostFailure, InitializationError};
use std::io::{self, Write};

/// Per-run logging state with a host-controlled output writer.
pub struct RunState {
    writer: Box<dyn Write + Send>,
    level: Level,
    colored: bool,
}

impl RunState {
    /// Construct a run with an explicit output writer and environment values.
    /// Missing, empty, or `"false"` values leave color enabled.
    pub fn with_writer(
        writer: impl Write + Send + 'static,
        no_color: Option<&str>,
        no_colour: Option<&str>,
    ) -> Self {
        let colored = !flag_enabled(no_color) && !flag_enabled(no_colour);
        Self {
            writer: Box::new(writer),
            level: Level::Info,
            colored,
        }
    }

    fn configure(&mut self) {
        self.level = Level::Info;
    }

    fn log(&mut self, level: Level, message: &str) -> Result<(), HostFailure> {
        if level > self.level {
            return Ok(());
        }
        let (label, style) = level.format();
        let prefix = if self.colored {
            format!("\x1b[{style}m{label}\x1b[0m")
        } else {
            label.to_owned()
        };
        let line = format!("{prefix} {message}\n");
        self.writer.write_all(line.as_bytes()).map_err(io_failure)?;
        self.writer.flush().map_err(io_failure)
    }
}

fn flag_enabled(value: Option<&str>) -> bool {
    matches!(value, Some(value) if !value.is_empty() && value != "false")
}

fn io_failure(error: io::Error) -> HostFailure {
    HostFailure::new(format!("logging output failed: {error}"))
}

fn initialize(configuration: &Configuration) -> Result<RunState, InitializationError> {
    for (key, _) in configuration.iter() {
        if key != "NO_COLOR" && key != "NO_COLOUR" {
            return Err(InitializationError::new(format!(
                "unknown logging configuration key `{key}`"
            )));
        }
    }
    let no_color = configured_flag(configuration, "NO_COLOR")?;
    let no_colour = configured_flag(configuration, "NO_COLOUR")?;
    Ok(RunState::with_writer(io::stdout(), no_color, no_colour))
}

fn configured_flag<'a>(
    configuration: &'a Configuration,
    key: &str,
) -> Result<Option<&'a str>, InitializationError> {
    configuration
        .get(key)
        .map(|value| {
            value
                .as_string()
                .map(|value| value.as_str())
                .ok_or_else(|| {
                    InitializationError::new(format!("configuration key `{key}` must be a String"))
                })
        })
        .transpose()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

impl Level {
    fn format(self) -> (&'static str, &'static str) {
        match self {
            Self::Emergency => ("EMRG", "1;41"),
            Self::Alert => ("ALRT", "1;41"),
            Self::Critical => ("CRIT", "1;41"),
            Self::Error => ("EROR", "1;31"),
            Self::Warning => ("WARN", "1;33"),
            Self::Notice => ("NTCE", "1;32"),
            Self::Info => ("INFO", "1;34"),
            Self::Debug => ("DEBG", "1;36"),
        }
    }
}

#[geam::provider(
    package = "logging",
    state = RunState,
    initialize = initialize,
    modules = [logging],
)]
pub struct Component;

#[geam::module(path = "logging")]
#[allow(ambiguous_associated_items)]
mod logging {
    use super::{Level, RunState};
    use geam::provider::{Call, HostResult, StringValue};

    #[allow(dead_code)]
    #[geam::custom(input = LogLevelInput)]
    enum LogLevel {
        Emergency,
        Alert,
        Critical,
        Error,
        Warning,
        Notice,
        Info,
        Debug,
    }

    #[allow(dead_code)]
    #[geam::custom(input = KeyInput)]
    enum Key {
        Level,
    }

    #[derive(Clone, PartialEq, Eq, Hash)]
    #[geam::external(name = "DoNotLeak")]
    struct DoNotLeak;

    #[geam::function]
    fn configure(#[geam::call] call: &mut Call<RunState>) -> () {
        call.state_mut().configure();
    }

    #[geam::function]
    fn erlang_log(
        #[geam::call] call: &mut Call<RunState>,
        level: LogLevelInput,
        message: StringValue,
    ) -> HostResult<DoNotLeak> {
        call.state_mut().log(level.into(), message.as_str())?;
        Ok(DoNotLeak)
    }

    #[geam::function]
    fn set_primary_config_level(
        #[geam::call] call: &mut Call<RunState>,
        _key: KeyInput,
        level: LogLevelInput,
    ) -> DoNotLeak {
        call.state_mut().level = level.into();
        DoNotLeak
    }

    impl From<LogLevelInput> for Level {
        fn from(level: LogLevelInput) -> Self {
            match level {
                LogLevelInput::Emergency => Self::Emergency,
                LogLevelInput::Alert => Self::Alert,
                LogLevelInput::Critical => Self::Critical,
                LogLevelInput::Error => Self::Error,
                LogLevelInput::Warning => Self::Warning,
                LogLevelInput::Notice => Self::Notice,
                LogLevelInput::Info => Self::Info,
                LogLevelInput::Debug => Self::Debug,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, RunState, flag_enabled, initialize};
    use geam::provider::Configuration;
    use std::collections::BTreeMap;
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct RecordingWriter(Arc<Mutex<Vec<u8>>>);

    impl RecordingWriter {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().expect("recorded bytes").clone())
                .expect("UTF-8 logging output")
        }
    }

    impl Write for RecordingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .expect("recorded bytes")
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FailingWriter {
        fail_write: bool,
    }

    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail_write {
                Err(io::Error::other("write unavailable"))
            } else {
                Ok(bytes.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush unavailable"))
        }
    }

    #[test]
    fn level_order_formatting_and_reconfiguration_are_exact() {
        let writer = RecordingWriter::default();
        let mut state = RunState::with_writer(writer.clone(), Some("1"), None);
        state.level = Level::Debug;
        for level in [
            Level::Emergency,
            Level::Alert,
            Level::Critical,
            Level::Error,
            Level::Warning,
            Level::Notice,
            Level::Info,
            Level::Debug,
        ] {
            state.log(level, "message").expect("record output");
        }
        assert_eq!(
            writer.text(),
            "EMRG message\nALRT message\nCRIT message\nEROR message\nWARN message\nNTCE message\nINFO message\nDEBG message\n"
        );

        state.level = Level::Error;
        state.log(Level::Warning, "hidden").expect("filtered");
        state.log(Level::Error, "visible").expect("record output");
        state.configure();
        assert_eq!(state.level, Level::Info);
        assert!(!state.colored);
        state.log(Level::Debug, "hidden").expect("filtered");
        state.log(Level::Info, "reset").expect("record output");
        assert!(writer.text().ends_with("EROR visible\nINFO reset\n"));
    }

    #[test]
    fn color_labels_match_original_formatter() {
        let writer = RecordingWriter::default();
        let mut state = RunState::with_writer(writer.clone(), None, Some("false"));
        assert!(state.colored);
        state.level = Level::Debug;
        for level in [
            Level::Emergency,
            Level::Alert,
            Level::Critical,
            Level::Error,
            Level::Warning,
            Level::Notice,
            Level::Info,
            Level::Debug,
        ] {
            state.log(level, "m").expect("record output");
        }
        assert_eq!(
            writer.text(),
            "\x1b[1;41mEMRG\x1b[0m m\n\x1b[1;41mALRT\x1b[0m m\n\x1b[1;41mCRIT\x1b[0m m\n\x1b[1;31mEROR\x1b[0m m\n\x1b[1;33mWARN\x1b[0m m\n\x1b[1;32mNTCE\x1b[0m m\n\x1b[1;34mINFO\x1b[0m m\n\x1b[1;36mDEBG\x1b[0m m\n"
        );
    }

    #[test]
    fn environment_values_follow_original_flag_rule() {
        assert!(!flag_enabled(None));
        assert!(!flag_enabled(Some("")));
        assert!(!flag_enabled(Some("false")));
        assert!(flag_enabled(Some("False")));
        assert!(flag_enabled(Some("0")));
        assert!(flag_enabled(Some("1")));
        assert!(!RunState::with_writer(io::sink(), None, Some("1")).colored);
    }

    #[test]
    fn configuration_rejects_unknown_and_non_string_values() {
        let unknown = Configuration::new(BTreeMap::from([("level".into(), "info".into())]));
        assert_eq!(
            initialize(&unknown).err().expect("invalid key").reason(),
            "unknown logging configuration key `level`"
        );
        for key in ["NO_COLOR", "NO_COLOUR"] {
            let invalid = Configuration::new(BTreeMap::from([(key.into(), true.into())]));
            assert_eq!(
                initialize(&invalid).err().expect("invalid value").reason(),
                format!("configuration key `{key}` must be a String")
            );
        }
        let valid = Configuration::new(BTreeMap::from([
            ("NO_COLOR".into(), "".into()),
            ("NO_COLOUR".into(), "1".into()),
        ]));
        assert!(!initialize(&valid).expect("valid flags").colored);
        assert!(
            initialize(&Configuration::empty())
                .expect("default colors")
                .colored
        );
    }

    #[test]
    fn output_failures_reach_the_call_boundary() {
        let mut write_failure =
            RunState::with_writer(FailingWriter { fail_write: true }, None, None);
        assert!(
            write_failure
                .log(Level::Info, "message")
                .expect_err("write failure")
                .to_string()
                .contains("logging output failed: write unavailable")
        );
        let mut flush_failure =
            RunState::with_writer(FailingWriter { fail_write: false }, None, None);
        assert!(
            flush_failure
                .log(Level::Info, "message")
                .expect_err("flush failure")
                .to_string()
                .contains("logging output failed: flush unavailable")
        );
    }
}
