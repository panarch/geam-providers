# gzlib 2.0.0 contract inventory

The unmodified Hex release is the source for all five Erlang externals. Every
declaration is in `src/gzlib.gleam`; `fixtures/upstream.sha256` records the
published package files used by the fixtures.

| Gleam function | Erlang FFI | Public result | Observable boundary |
| --- | --- | --- | --- |
| `compress(data)` | `gzlib_ffi:compress/1` | `Result(BitArray, Nil)` | Byte-aligned input yields a zlib stream; unaligned input yields `Error(Nil)`. Default compression level is platform-dependent. |
| `compress_custom(data, level:)` | `gzlib_ffi:compress/2` | `Result(BitArray, Nil)` | Levels 0–9 are accepted; other levels or unaligned input yield `Error(Nil)`. |
| `uncompress(data)` | `gzlib_ffi:uncompress/1` | `Result(BitArray, Nil)` | Valid zlib stream recovers its bytes. Missing/corrupt header, checksum, truncation, or unaligned input yields `Error(Nil)`. The Erlang implementation accepts bytes after the first complete stream and returns that stream's data. |
| `crc32(data)` | `gzlib_ffi:crc32/1` | `Result(Int, Nil)` | Exact unsigned CRC32 of aligned bytes; unaligned input yields `Error(Nil)`. |
| `crc32_continue(crc, data)` | `gzlib_ffi:crc32/2` | `Result(Int, Nil)` | Continues an unsigned CRC32 using a seed from 0 through 2^32−1; invalid seed or unaligned data yields `Error(Nil)`. |

The package README mentions `compress_with_level`, which is absent from the
2.0.0 Gleam source. Its source declaration is `compress_custom`.

Compression bytes and compression ratio can differ between native backends.
The contract tests require zlib interoperability and recovered input bytes;
they require exact CRC results. Malformed-input acceptance beyond the documented
cases is checked against the Erlang FFI and recorded where it affects callers.
