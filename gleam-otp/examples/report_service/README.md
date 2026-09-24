# Report service example

This Gleam application uses the unmodified `gleam_otp` 1.3.0 package with the
`geam-otp` provider. A named factory starts report workers. A dispatcher actor
accepts requests. A static `RestForOne` supervisor starts the factory before the
dispatcher, so a factory failure stops and restarts the dispatcher as well.
The example exposes `start`, `submit`, `worker_count`, and `shutdown` around
that process tree. `restart_factory` simulates a failure for the demonstration.
Its `Cargo.toml` explicitly selects the local `geam-otp` crate for `gleam_otp`.

From the repository root, install a Geam CLI built from the pinned commit and
run the example:

```sh
cargo install --git https://github.com/panarch/geam.git \
  --rev bd95b872c578df88f76ed5c4175fb1ea55ee8607 \
  --locked --root "$PWD/target/geam-cli" --bin geam geam
(cd gleam-otp/examples/report_service && gleam deps download)
(cd gleam-otp/examples/report_service && ../../../target/geam-cli/bin/geam prepare)
(cd gleam-otp/examples/report_service && ../../../target/geam-cli/bin/geam run)
```

The output must match [expected-output.txt](expected-output.txt): two successful
requests, a rejected nonpositive input, and another successful request after the
factory process is killed and restarted. The program checks the new factory and
dispatcher process identities and the factory's child count. Input and results
exist only in memory; interrupted requests are not automatically retried and
reports are not persisted.
