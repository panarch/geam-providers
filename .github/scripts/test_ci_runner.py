"""Process contracts for the Bash CI runner; no fixture build is mocked as passing."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LOGGER = """#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
name = Path(sys.argv[0]).name
args = sys.argv[1:]
if name == "cargo" and args and args[0] == "metadata":
    print(Path(os.environ["CI_FAKE_METADATA"]).read_text())
elif name == "cygpath":
    print(os.environ["CI_FAKE_NATIVE_PATH"] if args[0] == "-w" else os.environ["CI_FAKE_SHELL_PATH"])
else:
    with open(os.environ["CI_COMMAND_LOG"], "a") as log:
        row = {"program": name, "args": args, "cwd": os.getcwd()}
        if name == "python3":
            row["config"] = os.environ.get("GEAM_CONFIG")
        log.write(json.dumps(row) + "\\n")
    if name == os.environ.get("CI_FAIL_PROGRAM"):
        sys.exit(7)
"""


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="ci-runner space 한글 ")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        scripts = self.root / ".github/scripts"
        scripts.mkdir(parents=True)
        for name in ("ci-lib.sh", "run_ci.sh"):
            shutil.copyfile(ROOT / ".github/scripts" / name, scripts / name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for name in ("cargo", "gleam", "geam", "trace", "cygpath"):
            self.write_command(self.bin / name)
        self.owner = self.root / "toy"
        for fixture in ("gleam", "embedding"):
            (self.owner / "fixtures" / fixture).mkdir(parents=True)
        self.metadata_path = self.root / "metadata.json"
        self.declare_provider({})
        self.write_command(self.owner / "fixtures/gleam/build/geam/target/debug/geam_toy_fixture")
        self.log = self.root / "commands.jsonl"
        self.temp_parent = self.root / "external temp 한글"
        self.temp_parent.mkdir()
        (self.temp_parent / "keep.txt").write_text("caller-owned temporary file")
        self.env = {
            **os.environ, "PATH": str(self.bin) + os.pathsep + os.environ["PATH"],
            "CI_FAKE_METADATA": str(self.metadata_path), "CI_COMMAND_LOG": str(self.log),
            "GEAM_BIN": str(self.bin / "geam"), "TMPDIR": str(self.temp_parent),
        }
        for key in ("RUNNER_OS", "GITHUB_ACTIONS", "CI_FAIL_PROGRAM", "CI_INTEGRATIONS"):
            self.env.pop(key, None)

    def write_command(self, path):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(LOGGER.replace("#!/usr/bin/env python3", "#!" + sys.executable))
        path.chmod(0o755)

    def declare_provider(self, declaration):
        self.metadata_path.write_text(json.dumps({
            "workspace_members": ["toy"],
            "packages": [{
                "id": "toy", "name": "geam-toy", "manifest_path": str(self.owner / "Cargo.toml"),
                "metadata": {"geam": {"ci": declaration}},
            }],
        }))

    def write_integration(self, name, body):
        owner = self.root / "integrations" / name
        owner.mkdir(parents=True)
        (owner / "ci.json").write_text(json.dumps({
            "schema": 1, "script": "ci.sh", "providers": [],
            "runners": ["ubuntu-24.04"], "cache-workspaces": [],
        }))
        (owner / "ci.sh").write_text("#!/usr/bin/env bash\nset -euo pipefail\n" + body)
        return owner

    def run_ci(self, *arguments, **env):
        return subprocess.run(
            ["bash", str(self.root / ".github/scripts/run_ci.sh"), *arguments],
            cwd=self.root, text=True, capture_output=True, env={**self.env, **env},
        )

    def commands(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []

    def use_windows_jq_output(self):
        """Keep real jq filters and emulate jq.exe's documented text-mode stdout."""
        jq = shutil.which("jq")
        self.assertIsNotNone(jq)
        wrapper = self.bin / "jq"
        wrapper.write_text(
            "#!" + sys.executable + "\n"
            "import subprocess, sys\n"
            "args = sys.argv[1:]\n"
            "binary = '--binary' in args or '-b' in args\n"
            "args = [arg for arg in args if arg not in ('--binary', '-b')]\n"
            f"result = subprocess.run([{jq!r}, *args], input=sys.stdin.buffer.read(), capture_output=True)\n"
            "sys.stdout.buffer.write(result.stdout if binary else result.stdout.replace(b'\\n', b'\\r\\n'))\n"
            "sys.stderr.buffer.write(result.stderr)\n"
            "sys.exit(result.returncode)\n"
        )
        wrapper.chmod(0o755)

    def test_windows_jq_keeps_integration_batch_and_script_paths_exact(self):
        self.use_windows_jq_output()
        owner = self.write_integration("tool 한글", '"$CI_ROOT/bin/trace" "$CI_OWNER_DIR" "$1"\n')
        script = owner / "check source 한글.sh"
        (owner / "ci.sh").rename(script)
        declaration = json.loads((owner / "ci.json").read_text())
        declaration["script"] = script.name
        (owner / "ci.json").write_text(json.dumps(declaration))
        for arguments in (("integration", "tool 한글", "source"), ("integrations", "source")):
            with self.subTest(arguments=arguments):
                self.log.unlink(missing_ok=True)
                result = self.run_ci(
                    *arguments, RUNNER_OS="Windows",
                    CI_INTEGRATIONS=json.dumps([{"dir": "integrations/tool 한글"}]),
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.commands(), [{
                    "program": "trace", "args": [str(owner), "source"], "cwd": str(self.root),
                }])

    def test_windows_jq_keeps_provider_hook_and_executable_paths_exact(self):
        self.use_windows_jq_output()
        (self.bin / "cygpath").write_text('#!/usr/bin/env bash\nprintf "%s\\n" "$2"\n')
        script = self.owner / "fixtures/check hook 한글.sh"
        script.write_text('set -euo pipefail\n"$CI_ROOT/bin/trace" "$1" "${CI_FIXTURE_BINARY:-}"\n')
        self.declare_provider({
            "script": "fixtures/" + script.name, "script-phases": ["embedding", "executable"],
            "fixture-bin": "custom_fixture",
        })
        for phase in ("embedding", "standalone", "executable"):
            with self.subTest(phase=phase):
                self.log.unlink(missing_ok=True)
                result = self.run_ci("provider", "toy", phase, RUNNER_OS="Windows")
                self.assertEqual(result.returncode, 0, result.stderr)
                if phase == "standalone":
                    expected = {
                        "program": "geam", "args": ["run"],
                        "cwd": str(self.owner / "fixtures/gleam"),
                    }
                else:
                    binary = str(self.owner / "fixtures/gleam/build/geam/target/debug/custom_fixture.exe")
                    expected = {
                        "program": "trace", "args": [phase, binary if phase == "executable" else ""],
                        "cwd": str(self.root),
                    }
                self.assertEqual(self.commands(), [expected])
                self.assertEqual(list(self.temp_parent.glob("geam-ci.*")), [])

    def test_windows_jq_does_not_strip_control_characters_from_declared_paths(self):
        self.use_windows_jq_output()
        owner = self.write_integration("tool", '"$CI_ROOT/bin/trace" unexpected\n')
        declaration = json.loads((owner / "ci.json").read_text())
        declaration["script"] = "ci.sh\r"
        (owner / "ci.json").write_text(json.dumps(declaration))
        for arguments in (("integration", "tool", "source"), ("integrations", "source")):
            with self.subTest(arguments=arguments):
                result = self.run_ci(
                    *arguments, RUNNER_OS="Windows",
                    CI_INTEGRATIONS=json.dumps([{"dir": "integrations/tool\r"}]),
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("expected a relative repository path", result.stderr)
                self.assertEqual(self.commands(), [])

    def test_standard_provider_phases_use_expected_commands_and_working_directories(self):
        for phase, program, args, directory in (
            ("erlang", "gleam", ["run"], "gleam"),
            ("embedding", "cargo", ["run", "--locked"], "embedding"),
            ("standalone", "geam", ["run"], "gleam"),
        ):
            with self.subTest(phase=phase):
                self.log.unlink(missing_ok=True)
                result = self.run_ci("provider", "toy", phase)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.commands(), [{
                    "program": program, "args": args,
                    "cwd": str(self.owner / "fixtures" / directory),
                }])

    def test_hook_gets_exact_arguments_and_unselected_phases_use_default(self):
        script = self.owner / "fixtures/ci.sh"
        script.write_text('set -euo pipefail\n"$CI_ROOT/bin/trace" "$1" "" "space value" 한글\n')
        self.declare_provider({"script": "fixtures/ci.sh", "script-phases": ["standalone"]})
        result = self.run_ci("provider", "toy", "standalone")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.commands()[0]["args"], ["standalone", "", "space value", "한글"])
        self.log.unlink()
        result = self.run_ci("provider", "toy", "embedding")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.commands()[0]["program"], "cargo")

    def test_execution_failure_propagates_original_status_and_stops_later_work(self):
        script = self.owner / "fixtures/ci.sh"
        script.write_text('set -euo pipefail\n"$CI_ROOT/bin/trace" first\n"$CI_ROOT/bin/trace" second\n')
        self.declare_provider({"script": "fixtures/ci.sh", "script-phases": ["standalone"]})
        result = self.run_ci("provider", "toy", "standalone", CI_FAIL_PROGRAM="trace", GITHUB_ACTIONS="true")
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertEqual([command["args"] for command in self.commands()], [["first"]])
        self.assertIn("::endgroup::", result.stdout)

    def test_external_execution_directory_is_outside_fixture_and_removed(self):
        result = self.run_ci("provider", "toy", "executable")
        self.assertEqual(result.returncode, 0, result.stderr)
        command = self.commands()[0]
        self.assertEqual(command["program"], "geam_toy_fixture")
        self.assertEqual(Path(command["cwd"]).parent, self.temp_parent)
        self.assertFalse(Path(command["cwd"]).exists())
        self.assertEqual(list(self.temp_parent.glob("geam-ci.*")), [])
        self.assertEqual((self.temp_parent / "keep.txt").read_text(), "caller-owned temporary file")

    def test_external_directory_is_removed_after_failure(self):
        result = self.run_ci("provider", "toy", "executable", CI_FAIL_PROGRAM="geam_toy_fixture")
        self.assertEqual(result.returncode, 7)
        self.assertEqual(list(self.temp_parent.glob("geam-ci.*")), [])
        self.assertEqual((self.temp_parent / "keep.txt").read_text(), "caller-owned temporary file")

    def test_relative_temp_parent_is_normalized_before_changing_directory(self):
        parent = self.root / "relative temp 한글"
        parent.mkdir()
        self.write_integration("tool", 'cd "$CI_OWNER_DIR"\ncd "$CI_RUN_DIR"\n"$CI_ROOT/bin/trace" "$CI_RUN_DIR"\n')
        result = self.run_ci("integration", "tool", "executable", TMPDIR=parent.name)
        self.assertEqual(result.returncode, 0, result.stderr)
        command = self.commands()[0]
        self.assertEqual(Path(command["cwd"]).parent, parent)
        self.assertEqual(command["args"], [command["cwd"]])
        self.assertFalse(Path(command["cwd"]).exists())

    def test_temp_parent_inside_owner_is_rejected_and_only_runner_directory_is_removed(self):
        parent = self.owner / "fixtures/gleam"
        (parent / "keep.txt").write_text("caller-owned")
        result = self.run_ci("provider", "toy", "executable", TMPDIR=str(parent))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("outside its CI owner", result.stderr)
        self.assertEqual(self.commands(), [])
        self.assertEqual(list(parent.glob("geam-ci.*")), [])
        self.assertEqual((parent / "keep.txt").read_text(), "caller-owned")

    def test_batch_runs_only_selected_cases_and_preserves_space_unicode_owner(self):
        self.write_integration("first 한글", '"$CI_ROOT/bin/trace" "$CI_OWNER_DIR" "$1"\n')
        self.write_integration("unselected", 'exit 9\n')
        result = self.run_ci(
            "integrations", "source", CI_INTEGRATIONS=json.dumps([{"dir": "integrations/first 한글"}]),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.commands()[0]["args"], [str(self.root / "integrations/first 한글"), "source"])

    def test_batch_failure_stops_next_case(self):
        self.write_integration("first", '"$CI_ROOT/bin/trace" first\n')
        self.write_integration("second", '"$CI_ROOT/bin/trace" second\n')
        result = self.run_ci(
            "integrations", "source", CI_FAIL_PROGRAM="trace",
            CI_INTEGRATIONS=json.dumps([{"dir": "integrations/first"}, {"dir": "integrations/second"}]),
        )
        self.assertEqual(result.returncode, 7)
        self.assertEqual([command["args"] for command in self.commands()], [["first"]])

    def test_bad_batch_and_unknown_phase_fail_before_any_command(self):
        for arguments, env in (
            (("integrations", "source"), {"CI_INTEGRATIONS": "{}"}),
            (("provider", "toy", "unsupported"), {}),
        ):
            with self.subTest(arguments=arguments):
                result = self.run_ci(*arguments, **env)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.commands(), [])

    def test_script_cannot_be_a_symlink_or_escape_its_owner(self):
        outside = self.root / "outside.sh"
        outside.write_text('exit 0\n')
        for script_path in ("../outside.sh", "fixtures/link.sh"):
            self.declare_provider({"script": script_path, "script-phases": ["standalone"]})
            if script_path == "fixtures/link.sh":
                (self.owner / script_path).symlink_to(outside)
            result = self.run_ci("provider", "toy", "standalone")
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(self.commands(), [])

    def test_missing_geam_is_reported_before_execution(self):
        result = self.run_ci("provider", "toy", "standalone", GEAM_BIN="")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("GEAM_BIN", result.stderr)
        self.assertEqual(self.commands(), [])

    def test_httpc_hook_keeps_all_modules_and_native_windows_process_paths(self):
        self.use_windows_jq_output()
        self.write_command(self.bin / "python3")
        (self.bin / "cygpath").write_text(
            '#!/usr/bin/env bash\ncase "$1" in -u) printf "%s\\n" "$2";; '
            '-w) printf "native:%s\\n" "$2";; esac\n'
        )
        shutil.copyfile(ROOT / "gleam-httpc/fixtures/ci.sh", self.owner / "fixtures/ci.sh")
        self.declare_provider({"script": "fixtures/ci.sh", "script-phases": ["standalone", "executable"]})
        result = self.run_ci("provider", "toy", "standalone", RUNNER_OS="Windows", GEAM_CONFIG="")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([command["args"] for command in self.commands()], [
            ["../server.py", "native:" + self.env["GEAM_BIN"], "run", "--module", module,
             "--provider-config", "gleam_httpc=../config/provider.toml"]
            for module in ("geam_httpc_fixture", "httpc_redirect", "httpc_binary", "httpc_tls")
        ])
        self.log.unlink()
        result = self.run_ci("provider", "toy", "executable", RUNNER_OS="Windows")
        self.assertEqual(result.returncode, 0, result.stderr)
        command = self.commands()[0]
        self.assertEqual(command["args"], [
            "native:" + str(self.owner / "fixtures/server.py"),
            "native:" + str(self.owner / "fixtures/gleam/build/geam/target/debug/geam_toy_fixture.exe"),
        ])
        self.assertEqual(command["config"], "native:" + str(self.owner / "fixtures/config/runtime.toml"))
        self.assertEqual(Path(command["cwd"]).parent, self.temp_parent)
        self.assertFalse(Path(command["cwd"]).exists())

    def test_invalid_hook_declarations_do_not_fall_back_to_default_execution(self):
        for declaration in (
            False, {"script": False}, {"script-phases": False},
            {"script": "fixtures/ci.sh"}, {"script-phases": ["standalone"]},
            {"script": "fixtures/ci.sh", "script-phases": ["unsupported"]},
            {"script": "fixtures/ci.sh", "script-phases": ["standalone", "standalone"]},
            {"script": "fixtures/ci.sh", "script-phases": ["erlang"]},
        ):
            with self.subTest(declaration=declaration):
                self.declare_provider(declaration)
                result = self.run_ci("provider", "toy", "standalone")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("invalid provider CI declaration", result.stderr)
                self.assertEqual(self.commands(), [])

    def test_windows_path_boundary_and_executable_suffix(self):
        script = 'source "$1"; ci_native_path "$2"; ci_shell_path "$3"; ci_binary_path integrations/tool fixture'
        result = subprocess.run(
            ["bash", "-c", script, "path-check", str(self.root / ".github/scripts/ci-lib.sh"),
             str(self.root / "file space 한글"), "C:\\file space 한글"],
            text=True, capture_output=True,
            env={**self.env, "RUNNER_OS": "Windows", "CI_FAKE_NATIVE_PATH": "C:\\file space 한글",
                 "CI_FAKE_SHELL_PATH": str(self.root / "file space 한글")},
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), [
            "C:\\file space 한글", str(self.root / "file space 한글"),
            str(self.root / "integrations/tool/build/geam/target/debug/fixture.exe"),
        ])

    def test_named_step_failure_is_not_hidden_by_later_commands(self):
        script = (
            'set -euo pipefail; source "$1"; '
            'fail_then_continue() { "$CI_ROOT/bin/trace" first; "$CI_ROOT/bin/trace" second; }; '
            'ci_step "failure boundary" fail_then_continue; "$CI_ROOT/bin/trace" after'
        )
        result = subprocess.run(
            ["bash", "-c", script, "step-check", str(self.root / ".github/scripts/ci-lib.sh")],
            text=True, capture_output=True, env={**self.env, "CI_FAIL_PROGRAM": "trace"},
        )
        self.assertEqual(result.returncode, 7)
        self.assertEqual([command["args"] for command in self.commands()], [["first"]])

    def test_all_retained_bash_files_have_valid_syntax(self):
        paths = subprocess.check_output([
            "git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", "*.sh",
        ], cwd=ROOT).split(b"\0")
        for relative in filter(None, paths):
            path = ROOT / os.fsdecode(relative)
            with self.subTest(path=path):
                result = subprocess.run(["bash", "-n", str(path)], text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
