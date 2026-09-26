# Report service example

This Gleam application uses the unmodified `gleam_otp` 1.3.0 package with the
`geam-otp` provider. A named factory starts report workers. A dispatcher actor
accepts requests. A static `RestForOne` supervisor starts the factory before the
dispatcher, so a factory failure stops and restarts the dispatcher as well.
The example exposes `start`, `submit`, `worker_count`, and `shutdown` around
that process tree. `restart_factory` simulates a failure for the demonstration.
Its `Cargo.toml` explicitly selects the local `geam-otp` crate for `gleam_otp`.

From the repository root, build a Geam CLI workspace from the pinned commit and
run the example:

```sh
git init -q target/geam-source
git -C target/geam-source fetch --depth=1 https://github.com/panarch/geam.git cf5fb100c9220d9e30ff60a11d5bfbeb77f1ebef
git -C target/geam-source checkout --detach -q FETCH_HEAD
CARGO_TARGET_DIR="$PWD/target/geam-cli-build" \
  cargo build --manifest-path "$PWD/target/geam-source/Cargo.toml" \
  --locked --release --bin geam
(cd gleam-otp/examples/report_service && gleam deps download)
(cd gleam-otp/examples/report_service && ../../../target/geam-cli-build/release/geam prepare)
(cd gleam-otp/examples/report_service && ../../../target/geam-cli-build/release/geam run)
```

The output must match [expected-output.txt](expected-output.txt): two successful
requests, a rejected nonpositive input, and another successful request after the
factory process is killed and restarted. The program checks the new factory and
dispatcher process identities and the factory's child count. Input and results
exist only in memory; interrupted requests are not automatically retried and
reports are not persisted.
