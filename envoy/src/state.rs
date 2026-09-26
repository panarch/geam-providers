use geam::provider::{Configuration, HostFailure, InitializationError};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) value: String,
}

/// Environment values owned by one provider run.
pub struct RunState {
    entries: Arc<BTreeMap<String, Entry>>,
}

impl RunState {
    /// Start with a complete, host-supplied environment instead of reading the
    /// current process. This is also useful for deterministic embedding runs.
    pub fn from_values(values: BTreeMap<String, String>) -> Result<Self, InitializationError> {
        let mut entries = BTreeMap::new();
        for (name, value) in values {
            validate_entry(&name, &value)
                .map_err(|error| InitializationError::new(error.message().clone()))?;
            entries.insert(canonical_name(&name).into_owned(), Entry { name, value });
        }
        Ok(Self {
            entries: Arc::new(entries),
        })
    }

    pub(super) fn initialize(configuration: &Configuration) -> Result<Self, InitializationError> {
        Self::initialize_with_host(configuration, std::env::vars_os)
    }

    fn initialize_with_host<Entries>(
        configuration: &Configuration,
        host_environment: impl FnOnce() -> Entries,
    ) -> Result<Self, InitializationError>
    where
        Entries: IntoIterator<Item = (OsString, OsString)>,
    {
        if configuration.is_empty() {
            return Self::from_host_entries(host_environment());
        }
        for (key, _) in configuration.iter() {
            if key != "values" {
                return Err(InitializationError::new(format!(
                    "unknown envoy configuration key `{key}`"
                )));
            }
        }
        let values = configuration
            .get("values")
            .and_then(|value| value.as_table())
            .ok_or_else(|| {
                InitializationError::new("envoy configuration `values` must be a table")
            })?;
        let mut entries = BTreeMap::new();
        for (name, value) in values.iter() {
            let value = value.as_string().ok_or_else(|| {
                InitializationError::new(format!(
                    "envoy configuration `values.{name}` must be a String"
                ))
            })?;
            entries.insert(name.to_string(), value.to_string());
        }
        Self::from_values(entries)
    }

    fn from_host_entries(
        values: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Result<Self, InitializationError> {
        let mut entries = BTreeMap::new();
        for (name, value) in values {
            let name = name.into_string().map_err(|_| {
                InitializationError::new("host environment contains a non-Unicode name")
            })?;
            let value = value.into_string().map_err(|_| {
                InitializationError::new("host environment contains a non-Unicode value")
            })?;
            entries.insert(canonical_name(&name).into_owned(), Entry { name, value });
        }
        Ok(Self {
            entries: Arc::new(entries),
        })
    }

    pub(super) fn get(&self, name: &str) -> Result<Option<&str>, HostFailure> {
        validate_name(name)?;
        Ok(self
            .entries
            .get(canonical_name(name).as_ref())
            .map(|entry| entry.value.as_str()))
    }

    pub(super) fn set(&mut self, name: &str, value: &str) -> Result<(), HostFailure> {
        validate_entry(name, value)?;
        Arc::make_mut(&mut self.entries).insert(
            canonical_name(name).into_owned(),
            Entry {
                name: name.to_owned(),
                value: value.to_owned(),
            },
        );
        Ok(())
    }

    pub(super) fn unset(&mut self, name: &str) -> Result<(), HostFailure> {
        validate_name(name)?;
        Arc::make_mut(&mut self.entries).remove(canonical_name(name).as_ref());
        Ok(())
    }

    pub(super) fn snapshot(&self) -> Arc<BTreeMap<String, Entry>> {
        Arc::clone(&self.entries)
    }
}

fn validate_entry(name: &str, value: &str) -> Result<(), HostFailure> {
    validate_name(name)?;
    if value.contains('\0') {
        return Err(HostFailure::new("environment variable value contains NUL"));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), HostFailure> {
    if name.contains('=') {
        Err(HostFailure::new("environment variable name contains `=`"))
    } else if name.contains('\0') {
        Err(HostFailure::new("environment variable name contains NUL"))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn canonical_name(name: &str) -> Cow<'_, str> {
    Cow::Owned(name.to_ascii_uppercase())
}

#[cfg(not(windows))]
fn canonical_name(name: &str) -> Cow<'_, str> {
    Cow::Borrowed(name)
}

#[cfg(test)]
mod tests {
    use super::RunState;
    use geam::provider::{Configuration, HostFailure};
    use std::collections::BTreeMap;
    use std::ffi::OsString;

    #[test]
    fn explicit_values_are_isolated_and_mutations_change_one_run() {
        let values = BTreeMap::from([
            ("EMPTY".to_owned(), String::new()),
            ("LANG".to_owned(), "한국어".to_owned()),
        ]);
        let mut first = RunState::from_values(values.clone()).expect("valid values");
        let second = RunState::from_values(values).expect("independent state");
        assert_eq!(first.get("missing").expect("valid name"), None);
        assert_eq!(first.get("EMPTY").expect("valid name"), Some(""));
        assert_eq!(first.get("LANG").expect("valid name"), Some("한국어"));

        let old = first.snapshot();
        first.set("LANG", "새 값=1").expect("valid update");
        first
            .set("", "empty name")
            .expect("Erlang accepts empty names");
        first.unset("EMPTY").expect("valid removal");
        first
            .unset("missing")
            .expect("absent removal is idempotent");
        assert_eq!(old.len(), 2);
        assert_eq!(old.get("LANG").expect("old entry").value, "한국어");
        assert_eq!(first.get("LANG").expect("valid name"), Some("새 값=1"));
        assert_eq!(first.get("").expect("valid name"), Some("empty name"));
        assert_eq!(first.get("EMPTY").expect("valid name"), None);
        assert_eq!(second.get("LANG").expect("valid name"), Some("한국어"));
    }

    #[test]
    fn configuration_distinguishes_host_snapshot_from_explicit_empty_values() {
        let host_snapshot = RunState::initialize_with_host(&Configuration::empty(), || {
            [(OsString::from("HOST"), OsString::from("snapshot"))]
        })
        .expect("host values");
        assert_eq!(
            host_snapshot.get("HOST").expect("valid name"),
            Some("snapshot")
        );

        let empty = Configuration::new(BTreeMap::from([(
            "values".into(),
            Configuration::empty().into(),
        )]));
        assert_eq!(
            RunState::initialize(&empty)
                .expect("empty state")
                .snapshot()
                .len(),
            0
        );

        let configured = Configuration::new(BTreeMap::from([(
            "values".into(),
            Configuration::new(BTreeMap::from([("PORT".into(), "8080".into())])).into(),
        )]));
        assert_eq!(
            RunState::initialize(&configured)
                .expect("configured state")
                .get("PORT")
                .expect("valid name"),
            Some("8080")
        );
        let host = RunState::from_host_entries([(OsString::from("LANG"), OsString::from("🙂=ok"))])
            .expect("Unicode host snapshot");
        assert_eq!(host.get("LANG").expect("valid name"), Some("🙂=ok"));
    }

    #[test]
    fn default_initialization_reads_the_host_environment_once() {
        let path = std::env::var("PATH").expect("test runner has a Unicode PATH");
        let mut state = RunState::initialize(&Configuration::empty()).expect("host snapshot");
        assert_eq!(state.get("PATH").expect("valid name"), Some(path.as_str()));
        state.set("PATH", "provider-only").expect("valid update");
        assert_eq!(std::env::var("PATH").expect("host PATH"), path);
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_host_environment_fails_initialization() {
        use std::os::unix::ffi::OsStringExt;

        let invalid = OsString::from_vec(vec![0xff]);
        assert_eq!(
            RunState::from_host_entries([(invalid.clone(), OsString::from("value"))])
                .err()
                .expect("invalid name")
                .reason(),
            "host environment contains a non-Unicode name"
        );
        assert_eq!(
            RunState::from_host_entries([(OsString::from("NAME"), invalid)])
                .err()
                .expect("invalid value")
                .reason(),
            "host environment contains a non-Unicode value"
        );
    }

    #[cfg(windows)]
    #[test]
    fn non_unicode_host_environment_fails_initialization() {
        use std::os::windows::ffi::OsStringExt;

        let invalid = OsString::from_wide(&[0xd800]);
        assert_eq!(
            RunState::from_host_entries([(invalid.clone(), OsString::from("value"))])
                .err()
                .expect("invalid name")
                .reason(),
            "host environment contains a non-Unicode name"
        );
        assert_eq!(
            RunState::from_host_entries([(OsString::from("NAME"), invalid)])
                .err()
                .expect("invalid value")
                .reason(),
            "host environment contains a non-Unicode value"
        );
    }

    #[test]
    fn invalid_configuration_fails_before_execution() {
        let unknown = Configuration::new(BTreeMap::from([("other".into(), "x".into())]));
        assert_eq!(
            RunState::initialize(&unknown)
                .err()
                .expect("unknown key")
                .reason(),
            "unknown envoy configuration key `other`"
        );
        let wrong_table = Configuration::new(BTreeMap::from([("values".into(), "x".into())]));
        assert_eq!(
            RunState::initialize(&wrong_table)
                .err()
                .expect("wrong values type")
                .reason(),
            "envoy configuration `values` must be a table"
        );
        let wrong_value = Configuration::new(BTreeMap::from([(
            "values".into(),
            Configuration::new(BTreeMap::from([("PORT".into(), true.into())])).into(),
        )]));
        assert_eq!(
            RunState::initialize(&wrong_value)
                .err()
                .expect("wrong entry type")
                .reason(),
            "envoy configuration `values.PORT` must be a String"
        );
        for (name, value, reason) in [
            ("A=B", "x", "environment variable name contains `=`"),
            ("A", "x\0y", "environment variable value contains NUL"),
            ("A\0B", "x", "environment variable name contains NUL"),
        ] {
            assert_eq!(
                RunState::from_values(BTreeMap::from([(name.into(), value.into())]))
                    .err()
                    .expect("invalid configured entry")
                    .reason(),
                reason
            );
        }
    }

    #[test]
    fn invalid_runtime_inputs_fail_without_mutating_state() {
        let mut state = RunState::from_values(BTreeMap::new()).expect("empty state");
        for (name, reason) in [
            ("A=B", "environment variable name contains `=`"),
            ("A\0B", "environment variable name contains NUL"),
        ] {
            assert_eq!(state.get(name).expect_err("invalid name").message(), reason);
            assert_eq!(
                state.set(name, "x").expect_err("invalid name").message(),
                reason
            );
            assert_eq!(
                state.unset(name).expect_err("invalid name").message(),
                reason
            );
        }
        let failure: HostFailure = state.set("A", "x\0y").expect_err("NUL value");
        assert_eq!(failure.message(), "environment variable value contains NUL");
        assert!(state.snapshot().is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn windows_environment_names_are_ascii_case_insensitive() {
        let mut state = RunState::from_values(BTreeMap::new()).expect("empty state");
        state.set("PORT", "1").expect("first value");
        assert_eq!(state.get("port").expect("valid name"), Some("1"));
        state.set("Port", "2").expect("replace different casing");
        assert_eq!(state.snapshot().len(), 1);
        state.unset("pOrT").expect("remove different casing");
        assert!(state.snapshot().is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_environment_names_are_case_sensitive() {
        let mut state = RunState::from_values(BTreeMap::new()).expect("empty state");
        state.set("PORT", "1").expect("first value");
        state.set("port", "2").expect("second value");
        assert_eq!(state.get("PORT").expect("valid name"), Some("1"));
        assert_eq!(state.get("port").expect("valid name"), Some("2"));
        assert_eq!(state.snapshot().len(), 2);
    }
}
