# clip integration

Run [clip 1.2.2](https://hex.pm/packages/clip/1.2.2), a command-line argument
parser for Gleam, through Geam. This integration includes a file-search CLI
and a Rust embedding example that runs the same Gleam application.

The original Hex package is used unchanged. The examples combine the existing
`geam-argv`, `geam-filepath`, `geam-regexp` and `geam-simplifile` providers.
A small [application native adapter](fixtures/native/src/lib.rs) connects
the example's exit request to Geam's public execution API.

## Setup

Install Rust 1.96 or newer and Gleam. Build the Geam CLI from main commit
`e5e1f5f772c6f48369050bdf3ee35c7a324277e2` using the
[testing guide](../../docs/development/testing.md#dependency-preparation), and
set `GEAM_BIN` to the executable's absolute path. The Rust dependencies in
this integration use the same commit.

The examples use these locked Gleam package versions:

| Package | Version |
| --- | --- |
| clip | 1.2.2 |
| gleam_stdlib | 1.0.3 |
| argv | 1.1.0 |
| simplifile | 2.7.0 |
| filepath | 1.1.2 |
| gleam_regexp | 1.1.1 |

## File-search CLI

From the repository root, download the dependencies and build the
[application](fixtures/cli/src/geam_clip_search.gleam):

```sh
cd integrations/clip/fixtures/cli
gleam deps download
"$GEAM_BIN" prepare
"$GEAM_BIN" build
```

From that directory, create a sample file and search it:

```sh
printf 'Rust tools\nquiet line\nRUST async\n' > sample.txt
./build/geam/target/debug/geam_clip_search search -p Rust -i sample.txt
```

Output:

```text
sample.txt:1:Rust tools
sample.txt:3:RUST async
```

Count matching lines in the same file:

```sh
./build/geam/target/debug/geam_clip_search count -p Rust -i sample.txt
```

Output:

```text
sample.txt:2
```

Both commands accept one or more files. `search` prints `path:line:text`;
`count` prints each file's `path:count`.

| Option | Meaning |
| --- | --- |
| `--pattern PATTERN`, `-p PATTERN` | Required regular expression |
| `--ignore-case`, `-i` | Ignore letter case |
| `--help`, `-h` | Show root or command help |
| `--` | Treat subsequent arguments as file names, including names starting with a dash |

Show root or command help:

```sh
./build/geam/target/debug/geam_clip_search --help
./build/geam/target/debug/geam_clip_search search --help
```

On Windows, append `.exe` to the executable name; the shell examples use a
POSIX shell.

You can also run the application through the Geam CLI from the same directory:

```sh
"$GEAM_BIN" run -- search -p Rust -i sample.txt
```

### Behavior and limitations

- Results and help go to stdout; errors go to stderr. Unknown or leftover
  arguments are rejected.
- File order, line numbers, Unicode text, trailing spaces and the supplied
  path spelling are preserved. Input accepts LF and CRLF. An empty file has
  zero lines, and a final newline adds no extra line.
- All files are read before results are printed. A read failure produces no
  partial search output.
- Regular expressions follow the
  [geam-regexp backend contract](../../gleam-regexp/README.md).

The built executable and `geam run` use these exit codes:

| Code | Meaning |
| --- | --- |
| 0 | Successful search or count, no matches, or help |
| 1 | File read failure, including invalid UTF-8 |
| 2 | Invalid, missing or unexpected arguments, an empty path, or an invalid pattern |

## Rust embedding

The [Rust example](fixtures/embedding/src/main.rs) runs the same Gleam
application with host-supplied arguments, a host-owned output collector and
temporary files. It demonstrates repeated calls, independent execution states
and successful execution after errors and application exit requests.

The application returns `Completed(List(String))`, `Help(String)` or
`Failed(SearchError)`. Errors retain argument details, regexp compile details,
or the file path and `simplifile.FileError`. The example reads these results
through [Gleam functions](fixtures/embedding/gleam/src/geam_clip_embedding.gleam).
Calling `run` returns that structured result. Calling `main` applies the CLI
exit policy: the execution boundary returns `ExecutionOutcome::Exited(status)`
for application errors and `ExecutionOutcome::Returned` for success or help.
An exit ends that execution scope; the Rust host can start another scope with
the same module and state.

From the repository root:

```sh
for fixture in gleam cli embedding/gleam; do
  (cd "integrations/clip/fixtures/$fixture" && gleam deps download)
done
(cd integrations/clip/fixtures/embedding && cargo run --locked)
```

On success, the example prints:

```text
clip embedding: contracts, arguments, results, exit status, IO and state isolation passed
```

## Checks

The CLI checks require Python 3. After building the CLI and preparing the
embedding dependencies above, run these commands from the repository root:

```sh
python3 integrations/clip/fixtures/cli/check_cli.py --executable integrations/clip/fixtures/cli/build/geam/target/debug/geam_clip_search
(cd integrations/clip/fixtures/embedding && cargo test --locked)
```

Use the `.exe` path on Windows. The CLI checks create and remove their own
temporary files. The [package-contract fixture](fixtures/gleam/src/clip_contracts.gleam)
also covers clip's API beyond the search example. See the
[testing guide](../../docs/development/testing.md#package-integration-verification)
for package-contract checks, Erlang reference runs and full verification commands.

For the complete check sequence, install Bash, jq and Erlang as well, export
`GEAM_BIN` and the matching revision, then run from the repository root:

```sh
export GEAM_REV=e5e1f5f772c6f48369050bdf3ee35c7a324277e2
export GEAM_BIN="$PWD/target/geam-cli-build/release/geam"
bash .github/scripts/run_ci.sh integration clip
```

[ci.json](ci.json) registers this case and [ci.sh](ci.sh) contains its checks.
See [CI registration and execution](../../docs/development/ci.md) for individual
phases and Windows setup.
