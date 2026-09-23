# OTP native and service contracts

The input is the locked OTP 1.3.0 source, unchanged. Native implementations live
in the independent `geam-otp` crate. Tests do not substitute declarations,
Pid values, callback results, or supervisor success stubs in the upstream flow.
Owner tests separately use explicit local inputs to verify storage and malformed
native-data boundaries; those do not replace the original-source tests.
They live beside each implementation in [`../src/`](../src/). The original-source
contract target is [`../tests/original_otp.rs`](../tests/original_otp.rs)
and accesses the provider only through its public component contract.

| Original module and native | Provider owner and public boundary | Original-source evidence |
| --- | --- | --- |
| actor: `convert_system_message` | `actor.rs`, sealed native conversion and typed callback | `otp_service_fixture`, `otp_lifecycle`: get-state, suspend/resume, malformed system request |
| actor: `log_warning` | `actor.rs`, producer Charlist views and provider state | `otp_lifecycle`: exact unexpected-message warnings |
| actor: `erase` | `actor.rs`, exact source-native view retained as Dynamic | Actor state and status replies |
| system: `debug_state` | `system.rs`, immutable native external storage | Actor startup, exact default/debug-option comparison |
| system: `get_state` | `system.rs`, Rust-created typed reply callback, same mailbox | `otp_service_fixture`, `otp_lifecycle`: exact actor state |
| system: `erl_suspend` | `system.rs`, captured Pid/reference and empty reply callback | Suspend, status while suspended, subsequent resume |
| system: `erl_resume` | `system.rs`, same domain and receiver | Resume followed by successful requests |
| static: `make_erlang_child_spec` | `static_supervisor.rs`, retained typed nullary callback | `otp_mapped_child`: builder and `map_data`, captured transform on restart after the builder returns, unchanged start failure |
| static: `make_erlang_start_flags` | `static_supervisor.rs`, immutable flags | Original three restart strategies and three auto-shutdown policies in `otp_static_policies` |
| static: `make_timeout` | `static_supervisor.rs`, finite/infinite shutdown policy | Worker timeout and supervisor graceful shutdown |
| static: `erlang_start_link` | `static_supervisor.rs`, producer spawn/link/receive | Actual supervisor and child Pids, all restart strategies, significant termination and invalid child policies |
| static: `convert_erlang_start_error` | `static_supervisor.rs`, native-to-StartError target | `otp_supervision`: second child's InitFailed and first-child cleanup |
| factory: `make_erlang_child_spec` | `factory_supervisor.rs`, retained typed unary callback | Int→String and String→Int child data |
| factory: `make_erlang_start_flags` | `factory_supervisor.rs`, immutable flags | Original SimpleOneForOne startup and restart tolerance |
| factory: `make_timeout` | `factory_supervisor.rs`, shutdown policy | Original worker child specifications |
| factory: `unnamed_start` | `factory_supervisor.rs`, actual spawned supervisor and source startup error | Pid-backed factory in `otp_service_fixture`; invalid restart budget returns `InitFailed` in `otp_supervisor_edges` |
| factory: `named_start` | `factory_supervisor.rs`, shared producer name registry | Named factory, duplicate-name failure, and name reuse after invalid startup |
| factory: `convert_erlang_start_error` | `factory_supervisor.rs`, exact native StartError conversion | Duplicate-name InitFailed |
| factory: `pid_to_supervisor_handle` | `factory_supervisor.rs`, producer Pid retained in immutable handle | Pid-backed factory start-child/count |
| factory: `name_to_supervisor_handle` | `factory_supervisor.rs`, producer Name retained in immutable handle | `get_by_name` followed by start-child/count |
| factory: `erlang_start_child` | `factory_supervisor.rs`, exact argument restoration and typed Result2 | Two concrete argument/data combinations, child InitFailed, restart with original argument |
| factory: `erlang_count_children` | `factory_supervisor.rs`, request/reply through shared mailbox | Exact counts before/after child starts and restarts |

`static` and `factory` denote `gleam/otp/static_supervisor` and
`gleam/otp/factory_supervisor`. The actor/system names likewise denote the original
`gleam/otp` modules. Exact declaration text and source line numbers are in the
inventory.

The test-owned `otp_service_support.get_status` requests the original
`SystemMessage.GetStatus` path and checks the actual parent, mode, DebugState and
Dynamic state. OTP 1.3.0 does not expose a public get-status requester. This helper
is used by manual fixture tests; the deployed entrypoint uses the original
package APIs and needs only the `gleam_otp` provider selection.

Additional lifecycle evidence:

- `otp_init_timeout` advances a manual clock across the initializer's deadline
  and verifies that the actual child is gone.
- `otp_shutdown_timeout` observes a trapping child before the deadline and its
  forced termination at the deadline.
- `otp_supervision` verifies failed-start cleanup, graceful shutdown, retained
  restart arguments, a bounded restart budget, duplicate names and child errors.
- `otp_mapped_child` maps tuple child data to a String, checks the actual
  supervisor and new child Pids after restart, and proves that an initialization
  failure skips the transform. It also uses the original restart, timeout and
  significance property builders (`significant(False)` under `Never`).
- `otp_static_policies` runs `OneForOne`, `OneForAll`, and `RestForOne` with
  ordered child-start observations, checks a temporary sibling is not restarted,
  verifies that an inactive transient sibling's specification survives for a
  later group restart, runs `AnySignificant` and `AllSignificant` termination,
  and rejects significant children under `Never` or with permanent restart.
- `otp_lifetime` drops an execution with pending actor/system waits and a retained
  child callback. Its observer checks all four units, one service close, and one
  final release of the captured resource.
- `../tests/original_otp/cancellation.rs` replays each observed host notification in the normal
  request, lifecycle, supervision and timed-shutdown scenarios. It cancels while
  the notifying worker is still inside its poll, then checks the cancelled entry,
  every unit's completion and one service close. This covers cancellation errors
  returning through native waits as well as cancellation between worker polls.

Generated dynamic/prepared embedding and standalone run the representative
`otp_service_fixture` entrypoint. Their acceptance tests check repeated generation,
lock stability, exact repeated output and execution after source-free relocation.
Manual deterministic lifecycle tests provide the finer failure and clock evidence;
the deployed entrypoint is not a claim to execute every lifecycle branch.

The native inventory records all 22 original Erlang declarations with exact
Gleam signatures. Actor and system calls pass the source `Dynamic`, `Pid`,
`Subject`, and typed callback values through the shared `gleam_erlang` process
service. Child specifications retain callbacks for later restarts; static and
factory supervisors own the resulting child Pids and stop them when their
supervisor exits. Start failures preserve `actor.StartError` where declared;
invalid static supervisor flags and child policies return `InitFailed` before
any child starts. Unit tests next to each owner exercise malformed native data,
external value equality, hashing, inspection, and retained-value lifetimes.
