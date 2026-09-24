// Generated bindings compile this fixture's original Hex package at the
// provider crate's manifest path instead of the embedding crate's path.
#[allow(dead_code)]
#[path = "../fixtures/embedding/src/geam_bindings.rs"]
mod geam_bindings;

use geam::embedding::{BigInt, HostedModuleBuilder, HostedProject};
use geam::gleam_stdlib::{GleamStdlibRunState, IoOutput};
use geam::gleam_time::TimeSource;
use geam::{HostComponentProfile, HostFailure, HostProviderConfiguration};
use geam_birl::{RunState as BirlRunState, Source as BirlSource};
use std::time::{SystemTime, UNIX_EPOCH};

struct FixedTime;

impl TimeSource for FixedTime {
    fn system_time(&mut self) -> Result<SystemTime, HostFailure> {
        Ok(UNIX_EPOCH)
    }

    fn local_offset_seconds(&mut self) -> Result<i32, HostFailure> {
        Ok(32_400)
    }
}

struct ScriptedBirl {
    next_micros: i64,
    timezone: Option<String>,
}

impl BirlSource for ScriptedBirl {
    fn monotonic_micros(&mut self) -> BigInt {
        let current = self.next_micros;
        self.next_micros += 10;
        BigInt::from(current)
    }

    fn local_timezone(&mut self) -> Option<String> {
        self.timezone.clone()
    }
}

type Profile = geam_bindings::Profile<Vec<IoOutput>, FixedTime>;
type RunState = geam_bindings::RunState<Vec<IoOutput>, FixedTime>;

fn source_project() -> HostedProject<Profile> {
    HostedProject::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/embedding/gleam"),
        geam_bindings::ROOT_MODULE,
        geam_bindings::host_providers::<Vec<IoOutput>, FixedTime>,
    )
}

fn run_state(timezone: Option<&str>) -> RunState {
    let mut state = geam_bindings::RunStateInputs::<Vec<IoOutput>, FixedTime> {
        stdlib: GleamStdlibRunState::from_seed([0; 32]),
        time: FixedTime,
        birl: HostProviderConfiguration::empty(),
        gleam_regexp: HostProviderConfiguration::empty(),
    }
    .initialize()
    .expect("provider initializes");
    *<Profile as HostComponentProfile<geam_birl::Component>>::component_state(&mut state) =
        BirlRunState::with_source(ScriptedBirl {
            next_micros: 10,
            timezone: timezone.map(str::to_owned),
        });
    state
}

#[test]
fn original_birl_uses_injected_wall_time_offset_timezone_and_monotonic_order() {
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test executor");
    let host = geam::execution::TokioHost::new(executor.handle().clone());
    let program = source_project()
        .compile()
        .expect("original Hex package compiles");
    let (bindings, functions) =
        geam_bindings::bind(HostedModuleBuilder::new(program).expect("program plans"))
            .expect("source signatures bind");
    let mut module = bindings.seal().expect("module seals");
    let mut state = run_state(Some("Asia/Seoul"));
    let mut echo = Vec::new();

    executor
        .block_on(
            module.with_execution(&host, &mut state, &mut echo, async |scope| {
                for (timestamp, offset, expected) in [
                    (0, "Z", "Thursday"),
                    (-1, "Z", "Wednesday"),
                    (0, "-01:00", "Wednesday"),
                    (86_400_000_000_i64 - 1, "+00:00", "Thursday"),
                ] {
                    assert_eq!(
                        scope
                            .call(
                                &functions.weekday_at,
                                (BigInt::from(timestamp), offset.into())
                            )
                            .await
                            .expect("original weekday"),
                        expected
                    );
                }
                assert!(
                    scope
                        .call(&functions.sampled_order_is_lt, ())
                        .await
                        .expect("original compare prefers monotonic time")
                );
                assert_eq!(
                    scope
                        .call(&functions.sampled_difference, ())
                        .await
                        .expect("original difference uses microseconds"),
                    (BigInt::from(0), BigInt::from(10_000))
                );
                assert_eq!(
                    scope
                        .call(&functions.observed_offset, ())
                        .await
                        .expect("built-in local offset"),
                    "+09:00"
                );
                assert_eq!(
                    scope
                        .call(&functions.observed_timezone, ())
                        .await
                        .expect("provider local time-zone name"),
                    Some("Asia/Seoul".into())
                );
                assert_eq!(
                    scope
                        .call(&functions.observed_iso8601, ())
                        .await
                        .expect("combined local datetime"),
                    "1970-01-01T09:00:00.000+09:00"
                );
            }),
        )
        .expect("source execution completes");

    for timezone in [Some("Not/AZone"), None] {
        let mut state = run_state(timezone);
        executor
            .block_on(
                module.with_execution(&host, &mut state, &mut echo, async |scope| {
                    assert_eq!(
                        scope
                            .call(&functions.observed_timezone, ())
                            .await
                            .expect("original timezone validation"),
                        None
                    );
                }),
            )
            .expect("timezone absence is a successful run");
    }
}
