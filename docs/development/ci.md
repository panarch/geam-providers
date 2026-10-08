# CI registration and execution

CI discovers provider Cargo metadata and each integration's `ci.json`.
Execution commands belong to the target's Bash script. Adding a target with
the existing phase contract does not require editing the shared workflow or
selector.

The [testing baseline](testing.md) and [review policy](review-policy.md) still
define acceptance. A hook replaces an execution command; it cannot disable
formatting, Clippy, documentation, generated-binding checks, package checks,
tests, or the production crate's 100% line and region coverage gate.

## Provider registration

Add the crate to the root workspace, declare
`[package.metadata.geam.provider]`, and provide the standard
`fixtures/gleam` and `fixtures/embedding` projects. With no CI metadata,
validation uses Ubuntu, runs the embedding consumer and default Gleam module,
and executes the built binary outside its fixture. The binary name defaults
to the crate name with hyphens replaced by underscores, followed by
`_fixture`.

Optional `[package.metadata.geam.ci]` fields:

| Field | Meaning |
| --- | --- |
| `fixture-bin` | Standalone binary name |
| `example-dir` | Relative directory of an example with `expected-output.txt` |
| `erlang-oracle` | Run the original Erlang fixture; defaults to `false` |
| `runners` | Nonempty runner list; defaults to `["ubuntu-24.04"]` |
| `script` | Owner-relative Bash file for custom execution phases |
| `script-phases` | Phases handled by that script |
| `cache-workspaces` | Additional owner-relative `"workspace -> target"` entries |

For example, [argv](../../argv/Cargo.toml) declares:

```toml
[package.metadata.geam.ci]
runners = ["ubuntu-24.04", "macos-15", "windows-2025"]
erlang-oracle = true
script = "fixtures/ci.sh"
script-phases = ["erlang", "standalone", "executable"]
```

Declare `script` and a nonempty `script-phases` together. Supported phases are
`erlang`, `embedding`, `standalone`, and `executable`; an Erlang hook also
requires `erlang-oracle = true`. Other phases use the standard command.

The workflow runs preparation, source and Rust quality checks, binding checks,
embedding tests, standalone build, and coverage independently of hooks.
An `embedding` hook handles the consumer execution after its tests.
A `standalone` hook handles execution after `prepare`; an `executable` hook
runs after `build`. The script must retain the target's required scenarios.
[httpc's hook](../../gleam-httpc/fixtures/ci.sh), for example, owns its
loopback HTTP/HTTPS server and entry-module runs.

Root and embedding Cargo targets are cached by default. Declare additional
consumer caches only when needed. The workspace side of each additional entry
must contain `Cargo.toml`.

## Integration registration

Keep the unchanged Gleam package, fixtures, scripts and documentation together
under `integrations/NAME/`. An integration is not a production workspace
member. Every integration directory must register an existing Bash script with
all five fields in `ci.json`:

```json
{
  "schema": 1,
  "script": "ci.sh",
  "providers": ["geam-argv", "geam-filepath", "geam-regexp", "geam-simplifile"],
  "runners": ["ubuntu-24.04", "macos-15", "windows-2025"],
  "cache-workspaces": [
    "fixtures/gleam -> build/geam/target",
    "fixtures/cli -> build/geam/target",
    "fixtures/embedding -> target"
  ]
}
```

This is [clip's declaration](../../integrations/clip/ci.json).
`providers` names the existing Rust provider crates used by the case.
Discovery includes their transitive workspace and fixture dependencies.
`runners` declares the environments where this integration runs; it does not
infer support from provider runner lists. Cache paths are relative to the
integration directory. The pinned CLI cache is added once per OS job.

Use the same schema for new cases. Unknown fields, duplicate values, missing
scripts, unknown providers, malformed paths, or empty runner lists fail
discovery. Scripts must be regular `.sh` files inside their owner directory.
Paths use normalized repository-relative spelling.

Each integration script accepts these phases:

| Phase | Required work |
| --- | --- |
| `source` | Pin/lock checks, dependency download, original-source checksums, Gleam source checks, Rust formatting/Clippy/docs |
| `erlang` | Original Erlang reference scenarios |
| `embedding` | Generated-binding check, Rust tests and consumer execution |
| `standalone` | Prepare, run and build every standalone consumer |
| `executable` | Execute built consumers outside their fixtures |
| `all` | Run those five phases in order for local verification |

Commands remain ordinary Bash, with visible working directories and arguments.
Use `set -euo pipefail` and the common
[ci-lib.sh](../../.github/scripts/ci-lib.sh). See the complete
[clip](../../integrations/clip/ci.sh) and
[directories](../../integrations/directories/ci.sh) scripts.
A script owns its temporary resources and must propagate a failed check.
Use `ci_step` as a direct command to group output and retain failure behavior;
do not call it from an `if`, `&&`, or `||` condition.

## Local execution

Install Bash, jq, Rust, Gleam and Erlang, plus any tools required by the case
(such as Python 3 for clip and httpc). Build the pinned CLI as described in
[dependency preparation](testing.md#dependency-preparation). Export its
absolute path and the repository's revision before starting child scripts:

```sh
export GEAM_REV=08b5651661f83d59423ff20e271c62fb42a2f2ae
export GEAM_BIN="$PWD/target/geam-cli-build/release/geam"
bash .github/scripts/run_ci.sh integration clip
bash .github/scripts/run_ci.sh integration directories
```

On Windows, use Git Bash and the `geam.exe` path. The helpers convert paths
for native tools and append `.exe` to fixture binaries.

Run an individual integration phase with an extra argument, for example:

```sh
bash .github/scripts/run_ci.sh integration clip erlang
```

Provider execution phases use the same entry point:

```sh
bash .github/scripts/run_ci.sh provider argv erlang
bash .github/scripts/run_ci.sh provider argv embedding
(cd argv/fixtures/gleam && "$GEAM_BIN" prepare)
bash .github/scripts/run_ci.sh provider argv standalone
(cd argv/fixtures/gleam && "$GEAM_BIN" build)
bash .github/scripts/run_ci.sh provider argv executable
```

These commands run execution phases. They do not replace the provider's
mandatory quality, package, binding, test and coverage checks in the
[testing guide](testing.md).

The runner supplies `CI_ROOT` and `CI_OWNER_DIR` as absolute shell paths.
Executable phases and integration `all` also receive a fresh `CI_RUN_DIR`
outside the fixture; the runner removes only that directory on success or
failure. It normalizes the temporary path and rejects a `TMPDIR` inside the
target's owner directory. Provider executable hooks receive `CI_FIXTURE_BINARY`.
`GEAM_BIN` is an absolute shell path; use `ci_native_path` when passing paths
to native Windows processes.

## Selection and shared jobs

The [selector](../../.github/scripts/select_providers.py) compares the PR base
and checked-out head, or the previous and new main commits:

- Provider changes select that provider and its workspace/fixture dependents
  on every declared runner.
- Integration changes select that case. Provider changes also select cases
  whose declared dependency closure contains an affected provider.
- Pure workspace-member and lock-package additions can remain scoped.
  Relevant existing Cargo input changes, shared execution changes, unknown
  paths, or unavailable analysis use conservative broader selection.
- Each selected integration's runners form one job per OS. That job builds
  the CLI once and invokes only selected cases, phase by phase. Cache entries
  come only from those cases.

Root `quality` still runs the entire workspace on Ubuntu in ordinary CI.
Provider matrix scoping reduces repeated coverage and fixture jobs; it does
not skip workspace quality. Documentation-only changes retain the full
provider fallback and do not start unrelated integration jobs.

A new case or provider can stay scoped when its files, declarations and pure
Cargo additions are the only changes. Changing the common workflow, runner,
helper or selector intentionally selects all targets. Updating an existing
lockfile record or the common Geam pin also broadens selection. Therefore a
later provider PR can remain scoped after adopting this structure, provided
its diff meets those conditions.

Manual runs use the full provider matrix and omit integrations unless
`run_integrations` is enabled. Set both `coverage_provider` and
`coverage_runner` to a declared pair for focused coverage; that run skips
workspace quality and all integrations.

Run selector and Bash process-contract tests locally:

```sh
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
```

CI also installs actionlint 1.7.12 and ShellCheck 0.11.0 and requires workflow
and repository Bash lint. With those tools on PATH, use the same check locally:

```sh
bash .github/scripts/lint_ci.sh
```

The tests execute the actual workflow discovery block and temporary Git/Cargo
repositories. Process tests check dispatch, exact arguments, failure
propagation, external working directories and cleanup; they do not establish
passing builds or native Windows behavior. Native platform evidence comes
from the declared hosted jobs.
