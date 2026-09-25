# geam-simplifile

This crate implements the 19 Erlang externals of the unmodified
[`simplifile` 2.7.0](https://hex.pm/packages/simplifile/2.7.0) Gleam package
through Geam's public typed provider API. Add `simplifile` and its dependency
[`filepath` 1.1.2](https://hex.pm/packages/filepath/1.1.2) to the Gleam project,
then select both `geam-simplifile` and `geam-filepath` as Geam providers. The
supported upstream range is limited to 2.7.0 until other versions are tested.

The [standalone fixture](fixtures/gleam/) and
[embedding fixture](fixtures/embedding/) both compile the original Hex packages
without upstream changes. Geam is pinned to `main` commit
`b23d82a23d31c77eca749d18f67e73f0d9d952c7`. The provider implements the
host file system effects; the upstream Gleam package retains its public API and
the Gleam implementations of operations such as `read`, `write`, and recursive
copy.

File names and the current directory must be Unicode. Non-Unicode directory
entry names produce `Einval`. Non-byte-aligned bit arrays produce `Einval` for
write and append. Common host errors map to the corresponding `FileError`
constructors; unclassified host errors use `Unknown` with the host message.
`resolve` makes a path absolute lexically without requiring it to exist or
following symbolic links. The original external has no error result; if the
host cannot resolve its current directory, it leaves the input path unchanged.

On Unix, `FileInfo` uses native metadata. On Windows, its synthetic mode encodes
the host file type, assumes read bits, derives write bits from the readonly
flag, and sets execute bits for directories; it does not model Windows ACLs.
Stable Rust metadata does not expose Unix-style link count, inode, user ID,
group ID, device, or change time on Windows; those fields are zero. Windows
access and modification times use host file timestamps. Setting octal
permissions returns `Enotsup` on Windows because its permission model differs
from Unix mode bits.
Windows symbolic-link creation is subject to host privileges. The CI workflow
declares Ubuntu, macOS, and Windows jobs for this crate; see the
[testing guide](../docs/development/testing.md) for the actual verification
scope.

This crate can be used from the pinned Git source. A crates.io release remains
separate because Cargo cannot yet resolve all required Geam features from its
published registry metadata.
