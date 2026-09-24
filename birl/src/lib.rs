//! Native externals for the unmodified `birl` 2.0.0 package.

use geam::provider::{BigInt, Call, Configuration, InitializationError, StringValue};
use jiff::tz::TimeZone;
use num_integer::Integer;
use std::time::Instant;

/// Host-owned clock and local time-zone source for one provider run.
pub trait Source: Send + 'static {
    /// Microseconds since this source's stable monotonic origin.
    fn monotonic_micros(&mut self) -> BigInt;

    /// Local IANA time-zone name, if the host can determine one.
    fn local_timezone(&mut self) -> Option<String>;
}

/// The default source backed by the host operating system.
pub struct SystemSource {
    origin: Instant,
}

impl Default for SystemSource {
    /// Capture the monotonic origin for a provider run.
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Source for SystemSource {
    fn monotonic_micros(&mut self) -> BigInt {
        BigInt::from(self.origin.elapsed().as_micros())
    }

    fn local_timezone(&mut self) -> Option<String> {
        timezone_name(TimeZone::try_system())
    }
}

fn timezone_name<Error>(zone: Result<TimeZone, Error>) -> Option<String> {
    // The original external returns Option, so a failed system lookup is absence.
    match zone {
        Ok(zone) => zone.iana_name().map(|name| {
            if name == "UTC" {
                "Etc/UTC".to_owned()
            } else {
                name.to_owned()
            }
        }),
        Err(_) => None,
    }
}

/// Per-run native state. Embedding hosts can supply the same time-zone
/// configuration they use for Geam's `gleam_time` source.
pub struct RunState {
    source: Box<dyn Source>,
}

impl RunState {
    /// Construct a provider run with an explicit host source.
    pub fn with_source(source: impl Source) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

fn initialize(configuration: &Configuration) -> Result<RunState, InitializationError> {
    if let Some((key, _)) = configuration.iter().next() {
        return Err(InitializationError::new(format!(
            "unknown birl configuration key `{key}`"
        )));
    }
    Ok(RunState::with_source(SystemSource::default()))
}

fn weekday_from_micros(timestamp: BigInt, offset: BigInt) -> BigInt {
    let day = (timestamp + offset).div_floor(&BigInt::from(86_400_000_000u64));
    (day + BigInt::from(3u8)).mod_floor(&BigInt::from(7u8))
}

#[geam::provider(
    package = "birl",
    state = RunState,
    initialize = initialize,
    modules = [birl],
)]
pub struct Component;

#[geam::module(path = "birl")]
mod birl {
    use super::{BigInt, Call, RunState, StringValue, weekday_from_micros};

    #[geam::function]
    fn ffi_monotonic_now(#[geam::call] call: &mut Call<RunState>) -> BigInt {
        call.state_mut().source.monotonic_micros()
    }

    #[geam::function]
    fn ffi_weekday(timestamp_micros: BigInt, offset_micros: BigInt) -> BigInt {
        weekday_from_micros(timestamp_micros, offset_micros)
    }

    #[geam::function]
    fn local_timezone(#[geam::call] call: &mut Call<RunState>) -> Option<StringValue> {
        call.state_mut().source.local_timezone().map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::{Source, SystemSource, initialize, timezone_name, weekday_from_micros};
    use geam::provider::{BigInt, Configuration};
    use jiff::tz::{TimeZone, offset};
    use std::collections::BTreeMap;

    #[test]
    fn weekday_uses_local_calendar_day_and_monday_zero() {
        let scenarios: [(i64, i64, i64); 6] = [
            (0, 0, 3),
            (3 * 86_400_000_000, 0, 6),
            (-1, 0, 2),
            (0, -1, 2),
            (86_400_000_000 - 1, 1, 4),
            (0, 9 * 3_600_000_000, 3),
        ];
        for (timestamp, offset, expected) in scenarios {
            assert_eq!(
                weekday_from_micros(BigInt::from(timestamp), BigInt::from(offset)),
                BigInt::from(expected)
            );
        }
    }

    #[test]
    fn weekday_handles_integer_values_beyond_machine_range() {
        let weeks = BigInt::parse_bytes(b"123456789012345678901234567890", 10)
            .expect("valid integer fixture");
        let week_micros = BigInt::from(7u8) * BigInt::from(86_400_000_000u64);
        assert_eq!(
            weekday_from_micros(&weeks * &week_micros, BigInt::from(0)),
            BigInt::from(3)
        );
        assert_eq!(
            weekday_from_micros(-weeks * week_micros - 1, BigInt::from(0)),
            BigInt::from(2)
        );
    }

    #[test]
    fn system_zone_names_remain_optional_and_utc_is_a_birl_zone() {
        assert_eq!(
            timezone_name::<()>(Ok(TimeZone::get("Asia/Seoul").expect("known zone"))),
            Some("Asia/Seoul".to_owned())
        );
        assert_eq!(
            timezone_name::<()>(Ok(TimeZone::UTC)),
            Some("Etc/UTC".to_owned())
        );
        assert_eq!(timezone_name::<()>(Ok(TimeZone::fixed(offset(9)))), None);
        assert_eq!(timezone_name::<()>(Err(())), None);
    }

    #[test]
    fn configuration_accepts_only_the_documented_system_default() {
        assert!(initialize(&Configuration::empty()).is_ok());
        let config = Configuration::new(BTreeMap::from([("timezone".into(), "Asia/Seoul".into())]));
        let failure = initialize(&config).err().expect("unknown key fails");
        assert!(failure.to_string().contains("timezone"));
    }

    #[test]
    fn system_source_returns_monotonic_microseconds_and_a_timezone_result() {
        let mut source = SystemSource::default();
        let first = source.monotonic_micros();
        let second = source.monotonic_micros();
        assert!(second >= first);
        let _ = source.local_timezone();
    }
}
