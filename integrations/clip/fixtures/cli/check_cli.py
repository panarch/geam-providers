"""Check actual argument vectors, output streams, and application exit codes."""

import argparse
import os
import re
import subprocess
import tempfile
from pathlib import Path


ROOT_HELP = (
    "Usage: clip-search <search|count> --pattern PATTERN [-i] FILE...\n\n"
    "Commands:\n  search  Print matching lines\n  count   Print matching line counts\n\n"
    "Use --help after a command for its options.\n"
)
SEARCH_HELP = (
    "Usage: clip-search search --pattern PATTERN [-i] FILE...\n\n"
    "Options:\n  --pattern, -p PATTERN  Regular expression\n"
    "  --ignore-case, -i     Ignore letter case\n"
    "  --help, -h            Print this help\n\n"
    "Use -- before file names starting with a dash.\n"
)
COUNT_HELP = SEARCH_HELP.replace("clip-search search", "clip-search count")
CONTRACT_TRACE = (
    "arg.map\narg.try_map\narg.map\narg.try_map\n"
    "opt.map\nopt.try_map\napply\narg.fail\nopt.fail\n"
)


def check(command, arguments, cwd, stdout="", stderr="", code=0, geam_run=False):
    environment = os.environ.copy()
    environment["CARGO_TERM_COLOR"] = "never"
    result = subprocess.run(
        [*command, *arguments], cwd=cwd, capture_output=True,
        text=True, encoding="utf-8", timeout=60, env=environment,
    )
    application_stderr = result.stderr
    if geam_run:
        # Geam builds before launching the app. Keep that boundary explicit:
        # require the runner phase and Cargo's successful build completion,
        # then compare the complete application stderr after it, including any
        # unexpected runner/runtime diagnostic. No app output is stripped.
        marker = "geam: Starting standalone runner for geam_clip_search\n"
        if result.stderr.count(marker) != 1:
            raise AssertionError("missing Geam runner phase: " + result.stderr)
        _, build = result.stderr.split(marker)
        finished = re.search(r"^\s+Finished `dev` profile [^\n]*\n", build, re.MULTILINE)
        if finished is None:
            raise AssertionError("Geam runner build did not finish: " + build)
        application_stderr = build[finished.end():]
    # text=True translates the host's CRLF stream framing to LF. No stripping,
    # path rewriting, or substring matching is applied to application output.
    actual = (result.returncode, result.stdout, application_stderr)
    expected = (code, stdout, stderr)
    if actual != expected:
        raise AssertionError(
            "arguments={!r}\nexpected={!r}\nactual={!r}".format(arguments, expected, actual)
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument("--executable", type=Path)
    target.add_argument("--erlang-project", type=Path)
    target.add_argument("--geam-project", type=Path)
    parser.add_argument("--geam-bin", type=Path)
    parser.add_argument("--contracts", action="store_true", help="Check the package contract entry point and callback trace")
    args = parser.parse_args()
    if bool(args.geam_project) != bool(args.geam_bin):
        parser.error("--geam-project and --geam-bin must be provided together")
    if args.geam_project and args.contracts:
        parser.error("--contracts uses --executable or --erlang-project")
    if args.executable:
        command = [str(args.executable.resolve(strict=True))]
    elif args.geam_project:
        project = args.geam_project.resolve(strict=True)
        command = [str(args.geam_bin.resolve(strict=True)), "run", "--"]
    else:
        project = args.erlang_project.resolve(strict=True)
        beams = sorted((project / "build/dev/erlang").glob("*/ebin"))
        if not beams:
            raise ValueError("Build the original Erlang application before checking it")
        command = [
            "erl", "-noshell", "-pa", *[str(path) for path in beams],
            "-eval", ("geam_clip_fixture:main(), halt()." if args.contracts else
                      "geam_clip_search:main(), halt()."), "-extra",
        ]

    with tempfile.TemporaryDirectory(prefix="clip-cli-") as directory:
        root = Path(directory)
        if args.contracts:
            check(command, [], root, stdout=CONTRACT_TRACE)
            print("clip contracts: exact callback order and count passed")
            return
        (root / "alpha.txt").write_bytes("Rust tools\nquiet line\nRUST async\n한글 Rust\n".encode())
        (root / "notes space.txt").write_bytes(b"rust lower\nRust again")
        (root / "unicode 한글.txt").write_bytes("Rust café\n마지막 줄\n".encode())
        (root / "empty.txt").write_bytes(b"")
        (root / "blank.txt").write_bytes(b"\n")
        (root / "-dash.txt").write_bytes(b"Rust dash\n")
        (root / "--help").write_bytes(b"Rust literal help\n")
        (root / "line endings.txt").write_bytes(b"Rust first\r\nplain\r\nRust last  \r\n")
        (root / "binary.bin").write_bytes(b"\xff\x00")
        (root / "sub").mkdir()
        (root / "sub/nested.txt").write_bytes(b"Rust nested\n")
        if args.geam_project:
            # geam run owns the project cwd; pass absolute corpus paths instead
            # of relying on a working directory outside that project.
            alpha = str(root / "alpha.txt")
            unicode = str(root / "unicode 한글.txt")
            missing = str(root / "missing.txt")
            binary = str(root / "binary.bin")
            cases = [
                (["search", "-p", "Rust", unicode], unicode + ":1:Rust café\n", "", 0),
                (["count", "-p", "absent", alpha], alpha + ":0\n", "", 0),
                (["--help"], ROOT_HELP, "", 0),
                ([], "", "arguments: No subcommand provided\n", 2),
                (["search", "-p", "Rust", alpha, "--bogus"], "", "unexpected arguments: --bogus\n", 2),
                (["search", "-p", "(", missing], "", "invalid regular expression\n", 2),
                (["search", "-p", "Rust", missing], "", "cannot read " + missing + ": No such file or directory\n", 1),
                (["search", "-p", "Rust", alpha, missing], "", "cannot read " + missing + ": No such file or directory\n", 1),
                (["search", "-p", "Rust", binary], "", "cannot read " + binary + ": File not UTF-8 encoded\n", 1),
                (["search", "-p", "Rust", ""], "", "file path must not be empty\n", 2),
            ]
            for arguments, stdout, stderr, code in cases:
                check(command, arguments, project, stdout, stderr, code, geam_run=True)
            print("clip geam run: {} application exit cases passed".format(len(cases)))
            return
        cases = [
            (["search", "--pattern", "Rust", "alpha.txt"],
             "alpha.txt:1:Rust tools\nalpha.txt:4:한글 Rust\n", "", 0),
            (["search", "alpha.txt", "-p", "Rust", "-i"],
             "alpha.txt:1:Rust tools\nalpha.txt:3:RUST async\nalpha.txt:4:한글 Rust\n", "", 0),
            (["count", "-p", "Rust", "--ignore-case", "alpha.txt", "notes space.txt"],
             "alpha.txt:3\nnotes space.txt:2\n", "", 0),
            (["search", "-p", "Rust", "notes space.txt", "alpha.txt"],
             "notes space.txt:2:Rust again\nalpha.txt:1:Rust tools\nalpha.txt:4:한글 Rust\n", "", 0),
            (["search", "-p", "한글", "alpha.txt"], "alpha.txt:4:한글 Rust\n", "", 0),
            (["search", "-p", "Rust", "unicode 한글.txt"], "unicode 한글.txt:1:Rust café\n", "", 0),
            (["search", "-p", "Rust", "sub/nested.txt"], "sub/nested.txt:1:Rust nested\n", "", 0),
            (["search", "-p", "Rust", "line endings.txt"],
             "line endings.txt:1:Rust first\nline endings.txt:3:Rust last  \n", "", 0),
            (["search", "-p", "absent", "alpha.txt"], "", "", 0),
            (["count", "-p", "absent", "alpha.txt", "empty.txt"], "alpha.txt:0\nempty.txt:0\n", "", 0),
            (["search", "-p", "", "empty.txt"], "", "", 0),
            (["count", "-p", "", "alpha.txt", "notes space.txt", "blank.txt", "empty.txt"],
             "alpha.txt:4\nnotes space.txt:2\nblank.txt:1\nempty.txt:0\n", "", 0),
            (["search", "-p", "Rust", "--", "-dash.txt"], "-dash.txt:1:Rust dash\n", "", 0),
            (["search", "-p", "Rust", "--", "--help"], "--help:1:Rust literal help\n", "", 0),
            (["search", "-p", "-absent", "alpha.txt"], "", "", 0),
            (["--help"], ROOT_HELP, "", 0),
            (["-h"], ROOT_HELP, "", 0),
            (["search", "--help"], SEARCH_HELP, "", 0),
            (["count", "-h"], COUNT_HELP, "", 0),
            (["search", "-p", "--help", "alpha.txt"], SEARCH_HELP, "", 0),
            ([], "", "arguments: No subcommand provided\n", 2),
            (["unknown"], "", "arguments: No subcommand provided\n", 2),
            (["search", "alpha.txt"], "", "arguments: missing required arg: --pattern, -p\n", 2),
            (["search", "-p"], "", "arguments: missing required arg: --pattern, -p\n", 2),
            (["search", "-p", "Rust"], "", "arguments: must provide at least one valid value for: file\n", 2),
            (["search", "-p", "Rust", "alpha.txt", "--bogus"], "", "unexpected arguments: --bogus\n", 2),
            (["search", "-p", "Rust", "-p", "lower", "alpha.txt"], "", "unexpected arguments: -p lower\n", 2),
            (["search", "-p", "(", "missing.txt"], "", "invalid regular expression\n", 2),
            (["search", "-p", "Rust", "missing.txt"], "", "cannot read missing.txt: No such file or directory\n", 1),
            (["search", "-p", "Rust", "alpha.txt", "missing.txt"], "", "cannot read missing.txt: No such file or directory\n", 1),
            (["search", "-p", "Rust", "binary.bin"], "", "cannot read binary.bin: File not UTF-8 encoded\n", 1),
            (["search", "-p", "Rust", ""], "", "file path must not be empty\n", 2),
        ]
        if os.name == "nt":
            path = str(root / "sub/nested.txt")
            cases.append((["search", "-p", "Rust", path], path + ":1:Rust nested\n", "", 0))
        for arguments, stdout, stderr, code in cases:
            check(command, arguments, root, stdout, stderr, code)
        print("clip CLI: {} exact process cases passed".format(len(cases)))


if __name__ == "__main__":
    main()
