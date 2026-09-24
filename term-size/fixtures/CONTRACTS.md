# term_size 1.0.1 source contract

The unchanged Hex source has one Erlang-targeted external:

| Gleam source function | Erlang target | Source return |
| --- | --- | --- |
| `term_size.get()` | `term_size_ffi:terminal_size/0` | `Result(#(Int, Int), Nil)` with rows before columns |

`term_size.rows()` and `term_size.columns()` have Gleam bodies that call `get()`.
They are not separate provider functions. The Erlang backend calls `io:rows()`
and `io:columns()`; the JavaScript backend checks Node stdout or Deno console
size. Both report `Error(Nil)` when a size cannot be determined. The provider
uses the first available terminal size among the host process's stdout, stderr,
and stdin. A separate embedding output sink is not a process TTY.

The `upstream.sha256` file covers all eight files in the Hex 1.0.1 archive.
The original Gleam source and FFI are downloaded into each fixture by `gleam
deps download`, never copied or patched into this repository.
