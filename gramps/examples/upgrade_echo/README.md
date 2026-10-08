# HTTP Upgrade and compressed WebSocket echo

This example runs an application-owned session using unchanged gramps 6.0.1.
It parses a fixed Upgrade request, builds a 101 response and its WebSocket
accept key, decodes a masked compressed text message, and returns an unmasked
compressed echo that the peer decodes. It also exchanges ping/pong and close
frames and closes all four compression contexts.

The reusable [session module](src/upgrade_echo/session.gleam) accepts text and
returns the decoded echo bytes. The fixture and embedding consumer import this
same module as a local Gleam dependency. It operates on supplied bytes; an
application supplies its own transport and extension negotiation. The fixed
transcript selects client/server no-context-takeover.

From this directory, run the original Erlang implementation:

```sh
gleam deps download
gleam run
```

Run it with Geam `main` commit
`08b5651661f83d59423ff20e271c62fb42a2f2ae`:

```sh
geam prepare
geam run
geam build
```

The checked-in runner selects `geam-gramps` and `geam-crypto` from this
repository. After building, `build/geam/target/debug/upgrade_echo` can run
from another directory; use `upgrade_echo.exe` on Windows.

[expected-output.txt](expected-output.txt) records logical results, independent
of the compression backend's choice of compressed bytes.
