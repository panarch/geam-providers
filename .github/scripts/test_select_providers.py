"""Contract tests for the CI matrix selector's fail-closed decisions."""

import json
import contextlib
import io
import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

import select_providers as ci


ROWS = [
    {"crate": "geam-filepath", "dir": "filepath", "runners": ["ubuntu-24.04"]},
    {
        "crate": "geam-simplifile",
        "dir": "simplifile",
        "runners": ["ubuntu-24.04", "macos-15", "windows-2025"],
    },
    {"crate": "geam-platform", "dir": "platform", "runners": ["ubuntu-24.04"]},
]
DEPENDENCIES = {
    "geam-filepath": set(),
    "geam-simplifile": {"geam-filepath"},
    "geam-platform": set(),
}
ALL = set(DEPENDENCIES)


class SelectionTests(unittest.TestCase):
    def test_platform_change_selects_only_platform(self):
        selected, reason = ci.affected_providers(
            ["platform/src/lib.rs", "platform/fixtures/gleam/Cargo.lock"], ROWS, DEPENDENCIES
        )
        self.assertEqual(selected, {"geam-platform"})
        self.assertEqual(reason, "affected providers")

    def test_filepath_change_includes_simplifile_on_all_declared_runners(self):
        selected, _ = ci.affected_providers(["filepath/src/lib.rs"], ROWS, DEPENDENCIES)
        self.assertEqual(selected, {"geam-filepath", "geam-simplifile"})
        runners = {runner for row in ROWS if row["crate"] in selected for runner in row["runners"]}
        self.assertEqual(runners, {"ubuntu-24.04", "macos-15", "windows-2025"})

    def test_simplifile_change_does_not_select_its_dependency(self):
        selected, _ = ci.affected_providers(["simplifile/src/lib.rs"], ROWS, DEPENDENCIES)
        self.assertEqual(selected, {"geam-simplifile"})

    def test_documentation_only_uses_full_matrix(self):
        selected, reason = ci.affected_providers(
            ["README.md", "docs/development/testing.md"], ROWS, DEPENDENCIES
        )
        self.assertEqual(selected, ALL)
        self.assertEqual(reason, "no provider change")

    def test_no_provider_change_uses_full_matrix(self):
        selected, reason = ci.affected_providers([], ROWS, DEPENDENCIES)
        self.assertEqual(selected, ALL)
        self.assertEqual(reason, "no provider change")

    def test_integration_only_change_skips_provider_matrix(self):
        selected, reason = ci.affected_providers(
            ["integrations/directories/fixtures/gleam/gleam.toml", "README.md"],
            ROWS,
            DEPENDENCIES,
        )
        self.assertEqual(selected, set())
        self.assertEqual(reason, "integration-only change")

    def test_integration_and_provider_change_keeps_affected_providers(self):
        selected, reason = ci.affected_providers(
            ["integrations/directories/README.md", "filepath/src/lib.rs"],
            ROWS,
            DEPENDENCIES,
        )
        self.assertEqual(selected, {"geam-filepath", "geam-simplifile"})
        self.assertEqual(reason, "affected providers")

    def test_shared_and_unknown_paths_require_full_matrix(self):
        for path in (".github/workflows/ci.yml", ".github/scripts/select_providers.py", "LICENSE"):
            with self.subTest(path=path):
                selected, reason = ci.affected_providers([path], ROWS, DEPENDENCIES)
                self.assertEqual(selected, ALL)
                self.assertEqual(reason, "shared or unknown path changed")

    def test_new_member_selects_its_provider(self):
        selected, _ = ci.affected_providers(
            [
                "Cargo.toml",
                "Cargo.lock",
                "README.md",
                "docs/development/testing.md",
                "platform/Cargo.toml",
            ],
            ROWS,
            DEPENDENCIES,
            added_members={"platform"},
        )
        self.assertEqual(selected, {"geam-platform"})

    def test_non_provider_member_or_changed_shared_cargo_requires_full_matrix(self):
        cases = [
            {"added_members": {"tools"}},
            {"manifest_safe": False},
            {"lock_safe": False},
        ]
        for options in cases:
            with self.subTest(options=options):
                selected, _ = ci.affected_providers(
                    ["Cargo.toml", "Cargo.lock"], ROWS, DEPENDENCIES, **options
                )
                self.assertEqual(selected, ALL)


class CargoChangeTests(unittest.TestCase):
    MANIFEST = """[workspace]
members = ["filepath"]
resolver = "3"

[workspace.dependencies]
geam = { git = "https://example.test/geam", rev = "old" }
"""
    LOCK = """version = 4

[[package]]
name = "geam-filepath"
version = "0.1.0"

[[package]]
name = "geam"
version = "0.1.0"
"""

    def test_only_new_members_and_dependencies_are_scoped(self):
        new = self.MANIFEST.replace(
            'members = ["filepath"]', 'members = ["filepath", "platform"]'
        ) + 'regex = "1.12"\n'
        self.assertEqual(ci.manifest_additions(self.MANIFEST, new), {"platform"})
        changed_existing = new.replace('rev = "old"', 'rev = "new"')
        self.assertIsNone(ci.manifest_additions(self.MANIFEST, changed_existing))
        self.assertIsNone(ci.manifest_additions(self.MANIFEST, self.MANIFEST.replace('"filepath"', '"platform"')))

    def test_existing_lock_records_must_be_identical(self):
        added = self.LOCK + '\n[[package]]\nname = "geam-platform"\nversion = "0.1.0"\n'
        self.assertTrue(ci.lock_additions_only(self.LOCK, added))
        self.assertFalse(ci.lock_additions_only(self.LOCK, added.replace('name = "geam"', 'name = "geam-core"')))
        self.assertFalse(ci.lock_additions_only(self.LOCK, self.LOCK.replace('version = "0.1.0"', 'version = "0.2.0"')))

    def test_directories_runs_for_relevant_paths_only(self):
        for path in (
            "integrations/directories/fixtures/gleam/gleam.toml",
            "envoy/src/lib.rs",
            "filepath/Cargo.toml",
            "platform/src/lib.rs",
            "simplifile/src/lib.rs",
            ".github/workflows/ci.yml",
            ".github/scripts/select_providers.py",
        ):
            with self.subTest(path=path):
                self.assertTrue(ci.run_directories_for_changes([path]))
        for path in (
            "integrations/another/fixtures/gleam/gleam.toml",
            "logging/src/lib.rs",
            "docs/development/testing.md",
            "Cargo.lock",
            "README.md",
        ):
            with self.subTest(path=path):
                self.assertFalse(ci.run_directories_for_changes([path]))
        self.assertTrue(ci.run_directories_for_changes(["logging/src/lib.rs", "envoy/src/lib.rs"]))
        self.assertFalse(ci.run_directories_for_changes([]))

    def test_directories_root_manifest_checks_only_relevant_workspace_inputs(self):
        original = self.MANIFEST + 'reqwest = "0.13"\n'
        another_member = original.replace('members = ["filepath"]', 'members = ["filepath", "logging"]')
        another_dependency = original.replace('reqwest = "0.13"', 'reqwest = "0.14"')
        comment_only = original.replace('reqwest = "0.13"', '# unrelated note\nreqwest = "0.13"')
        changed_geam = original.replace('rev = "old"', 'rev = "new"')
        for changed in (another_member, another_dependency, comment_only):
            self.assertFalse(ci.run_directories_for_changes(
                ["Cargo.toml"], original, changed, {"geam"}
            ))
        self.assertTrue(ci.run_directories_for_changes(
            ["Cargo.toml"], original, changed_geam, {"geam"}
        ))
        with self.assertRaises(ValueError):
            ci.run_directories_for_changes(["Cargo.toml"])

    def test_integration_flags_are_independent(self):
        cases = [
            (["integrations/clip/README.md"], (False, True)),
            (["integrations/directories/README.md"], (True, False)),
            (["integrations/clip/README.md", "integrations/directories/README.md"], (True, True)),
            (["argv/src/lib.rs"], (False, True)),
            (["gleam-regexp/fixtures/embedding/src/main.rs"], (False, True)),
            (["filepath/src/lib.rs"], (True, True)),
            (["simplifile/Cargo.toml"], (True, True)),
            (["envoy/src/lib.rs"], (True, False)),
            (["platform/src/lib.rs"], (True, False)),
            (["logging/src/lib.rs", "docs/development/testing.md"], (False, False)),
            (["integrations/unrelated/README.md", "Cargo.lock"], (False, False)),
            ([], (False, False)),
        ]
        cases.extend(([path], (True, True)) for path in ci.INTEGRATION_CI_PATHS)
        for paths, expected in cases:
            with self.subTest(paths=paths):
                self.assertEqual((
                    ci.run_directories_for_changes(paths),
                    ci.run_clip_for_changes(paths),
                ), expected)

    def test_clip_root_manifest_uses_its_workspace_dependencies(self):
        original = self.MANIFEST + 'reqwest = "0.13"\n'
        for changed in (
            original.replace('members = ["filepath"]', 'members = ["filepath", "logging"]'),
            original.replace('reqwest = "0.13"', 'reqwest = "0.14"'),
        ):
            self.assertFalse(ci.run_clip_for_changes(["Cargo.toml"], original, changed, {"geam"}))
        self.assertTrue(ci.run_clip_for_changes(
            ["Cargo.toml"], original, original.replace('rev = "old"', 'rev = "new"'), {"geam"}
        ))
        self.assertTrue(ci.run_clip_for_changes(
            ["Cargo.toml"], original, original.replace('resolver = "3"', 'resolver = "2"'), {"geam"}
        ))
        regex = original + 'regex = "1.12"\n'
        self.assertTrue(ci.run_clip_for_changes(
            ["Cargo.toml"], regex, regex.replace('regex = "1.12"', 'regex = "1.13"'),
            {"geam", "regex", "regex-syntax"},
        ))
        self.assertFalse(ci.run_directories_for_changes(
            ["Cargo.toml"], regex, regex.replace('regex = "1.12"', 'regex = "1.13"'), {"geam"},
        ))
        with self.assertRaises(ValueError):
            ci.run_clip_for_changes(["Cargo.toml"])

    def test_unrecognized_root_dependency_format_requires_full_analysis_fallback(self):
        # An unrecognized layout must not silently discard revision changes.
        before = self.MANIFEST.replace(
            'geam = { git = "https://example.test/geam", rev = "old" }',
            'geam = {\n  git = "https://example.test/geam",\n  rev = "old"\n}',
        )
        for selector in (ci.run_directories_for_changes, ci.run_clip_for_changes):
            with self.subTest(selector=selector.__name__), self.assertRaises(ValueError):
                selector(["Cargo.toml"], before, before.replace('"old"', '"new"'), {"geam"})


class RepositoryTests(unittest.TestCase):
    def test_directories_workspace_dependencies_include_geam(self):
        self.assertEqual(ci.directories_workspace_dependencies(), {"geam"})

    def test_clip_workspace_dependencies_include_geam(self):
        self.assertEqual(ci.clip_workspace_dependencies(), {"geam", "regex", "regex-syntax"})

    def test_current_workspace_and_fixtures_expose_filepath_dependency(self):
        metadata = json.loads(
            subprocess.check_output(
                ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
                cwd=ci.ROOT,
            )
        )
        graph = ci.provider_dependencies(ROWS, metadata, ci.ROOT, ci.fixture_manifests(ci.ROOT))
        self.assertIn("geam-filepath", graph["geam-simplifile"])

    def test_fixture_only_provider_dependency_is_detected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "first/fixtures/gleam/Cargo.toml"
            manifest.parent.mkdir(parents=True)
            (root / "second").mkdir()
            manifest.write_text('other = { path = "../../../second" }\n')
            rows = [
                {"crate": "first", "dir": "first"},
                {"crate": "second", "dir": "second"},
            ]
            metadata = {
                "workspace_members": ["first-id", "second-id"],
                "packages": [
                    {"id": "first-id", "name": "first", "dependencies": []},
                    {"id": "second-id", "name": "second", "dependencies": []},
                ],
            }
            graph = ci.provider_dependencies(
                rows, metadata, root, ["first/fixtures/gleam/Cargo.toml"]
            )
            self.assertEqual(graph["first"], {"second"})
            manifest.write_text("other = { path = '../../../second' }\n")
            with self.assertRaises(ValueError):
                ci.provider_dependencies(rows, metadata, root, ["first/fixtures/gleam/Cargo.toml"])

    def test_rename_reports_both_paths_and_missing_diff_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.check_call(["git", "init", "-q"], cwd=root)
            subprocess.check_call(["git", "config", "user.name", "CI test"], cwd=root)
            subprocess.check_call(["git", "config", "user.email", "ci@example.test"], cwd=root)
            old = root / "integrations/directories/old.txt"
            old.parent.mkdir(parents=True)
            old.write_text("fixture\n")
            subprocess.check_call(["git", "add", "."], cwd=root)
            subprocess.check_call(["git", "commit", "-qm", "initial"], cwd=root)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip()
            (root / "integrations/clip").mkdir()
            subprocess.check_call([
                "git", "mv", "integrations/directories/old.txt", "integrations/clip/new.txt",
            ], cwd=root)
            subprocess.check_call(["git", "commit", "-qm", "rename"], cwd=root)
            head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip()
            paths = ci.changed_paths(base, head, root)
            self.assertEqual(set(paths), {"integrations/directories/old.txt", "integrations/clip/new.txt"})
            self.assertTrue(ci.run_directories_for_changes(paths))
            self.assertTrue(ci.run_clip_for_changes(paths))
            with self.assertRaises(ValueError):
                ci.changed_paths("0" * 40, head, root)


class SelectorOutputTests(unittest.TestCase):
    def select(self, paths=None, error=None):
        metadata = {"workspace_members": [], "packages": []}
        stdout = io.StringIO()
        with (
            patch.dict(os.environ, {"PROVIDER_ROWS": json.dumps(ROWS)}),
            patch.object(ci, "changed_paths", return_value=paths, side_effect=error),
            patch.object(ci, "fixture_manifests", return_value=[]),
            patch.object(ci.subprocess, "check_output", return_value=json.dumps(metadata).encode()),
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            ci.main()
        return json.loads(stdout.getvalue())

    def test_clip_only_cli_output_skips_providers_and_directories(self):
        self.assertEqual(self.select(["integrations/clip/README.md"]), {
            "providers": [], "run_directories": False, "run_clip": True,
        })

    def test_directories_only_cli_output_skips_clip(self):
        self.assertEqual(self.select(["integrations/directories/README.md"]), {
            "providers": [], "run_directories": True, "run_clip": False,
        })

    def test_diff_failure_runs_every_provider_and_integration(self):
        for error in (ValueError("missing endpoint"), subprocess.CalledProcessError(1, "git")):
            with self.subTest(error=error):
                self.assertEqual(self.select(error=error), {
                    "providers": sorted(ALL), "run_directories": True, "run_clip": True,
                })


class WorkflowOutputTests(unittest.TestCase):
    def matrix(self, root=ci.ROOT, **env):
        # Execute the workflow's actual Bash rather than a copy of its dispatch logic.
        content = (ci.ROOT / ".github/workflows/ci.yml").read_text()
        step = content.split("      - name: Build provider matrix from Cargo metadata\n", 1)[1]
        script = step.split("        run: |\n", 1)[1].split("\n  quality:", 1)[0]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            result = subprocess.run(
                ["bash", "-e", "-o", "pipefail", "-c", textwrap.dedent(script)],
                cwd=root, text=True, capture_output=True,
                env={**os.environ, "GITHUB_WORKSPACE": str(root), "GITHUB_OUTPUT": str(output),
                     "CI_EVENT": "workflow_dispatch", "COVERAGE_PROVIDER": "", "COVERAGE_RUNNER": "",
                     "RUN_INTEGRATIONS": "false", **env},
            )
            values = dict(line.split("=", 1) for line in output.read_text().splitlines()) if output.exists() else {}
        return result, values

    def test_manual_integrations_flag_controls_both_cases(self):
        for enabled in ("true", "false"):
            with self.subTest(enabled=enabled):
                result, values = self.matrix(RUN_INTEGRATIONS=enabled)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(values["coverage_only"], "false")
                self.assertEqual(values["run_directories"], enabled)
                self.assertEqual(values["run_clip"], enabled)
                self.assertEqual(values["has_targets"], "true")

    def test_focused_coverage_skips_integrations_even_when_requested(self):
        result, values = self.matrix(
            COVERAGE_PROVIDER="geam-argv", COVERAGE_RUNNER="ubuntu-24.04", RUN_INTEGRATIONS="true",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(values["coverage_only"], "true")
        self.assertEqual(values["run_directories"], "false")
        self.assertEqual(values["run_clip"], "false")
        self.assertEqual([target["crate"] for target in json.loads(values["targets"])["include"]], ["geam-argv"])

    def test_invalid_focused_target_fails(self):
        result, _ = self.matrix(COVERAGE_PROVIDER="geam-argv", COVERAGE_RUNNER="invalid")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("No declared coverage target", result.stderr)

    def test_actual_pull_request_diff_controls_workflow_flags_and_provider_rows(self):
        cases = [
            (["integrations/clip/README.md"], ("false", "true"), "false"),
            (["integrations/directories/README.md"], ("true", "false"), "false"),
            (["integrations/clip/README.md", "integrations/directories/README.md"], ("true", "true"), "false"),
            (["invalid-diff"], ("true", "true"), "true"),
        ]
        for paths, flags, has_targets in cases:
            with self.subTest(paths=paths), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "Cargo.toml").write_text(
                    '[workspace]\nmembers = ["argv"]\nresolver = "3"\n\n[workspace.dependencies]\n'
                )
                for provider in set(ci.DIRECTORIES_PROVIDERS + ci.CLIP_PROVIDERS):
                    crate = root / provider
                    (crate / "src").mkdir(parents=True)
                    (crate / "src/lib.rs").write_text("")
                    (crate / "Cargo.toml").write_text(
                        '[package]\nname = "geam-' + provider + '"\nversion = "0.1.0"\n'
                        '[package.metadata.geam.provider]\ngleam-package = "' + provider + '"\n'
                        '[dependencies]\n'
                    )
                scripts = root / ".github/scripts"
                scripts.mkdir(parents=True)
                shutil.copyfile(ci.ROOT / ".github/scripts/select_providers.py", scripts / "select_providers.py")
                subprocess.check_call(["cargo", "generate-lockfile", "--offline"], cwd=root)
                subprocess.check_call(["git", "init", "-q"], cwd=root)
                subprocess.check_call(["git", "config", "user.name", "CI test"], cwd=root)
                subprocess.check_call(["git", "config", "user.email", "ci@example.test"], cwd=root)
                subprocess.check_call(["git", "add", "."], cwd=root)
                subprocess.check_call(["git", "commit", "-qm", "base"], cwd=root)
                base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip()
                for path in paths:
                    changed = root / path
                    changed.parent.mkdir(parents=True, exist_ok=True)
                    changed.write_text("integration\n")
                subprocess.check_call(["git", "add", "."], cwd=root)
                subprocess.check_call(["git", "commit", "-qm", "change"], cwd=root)
                head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip()
                result, values = self.matrix(
                    root=root, CI_EVENT="pull_request", GITHUB_SHA=head,
                    CI_BASE_SHA="missing" if paths == ["invalid-diff"] else base,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((values["run_directories"], values["run_clip"]), flags)
                self.assertEqual(values["has_targets"], has_targets)
                self.assertEqual(values["coverage_only"], "false")


if __name__ == "__main__":
    unittest.main()
