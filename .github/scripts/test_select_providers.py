"""Contract tests for the CI matrix selector's fail-closed decisions."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

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


class RepositoryTests(unittest.TestCase):
    def test_directories_workspace_dependencies_include_geam(self):
        self.assertEqual(ci.directories_workspace_dependencies(), {"geam"})

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
            (root / "old.txt").write_text("fixture\n")
            subprocess.check_call(["git", "add", "old.txt"], cwd=root)
            subprocess.check_call(["git", "commit", "-qm", "initial"], cwd=root)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip()
            subprocess.check_call(["git", "mv", "old.txt", "new.txt"], cwd=root)
            subprocess.check_call(["git", "commit", "-qm", "rename"], cwd=root)
            head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip()
            self.assertEqual(set(ci.changed_paths(base, head, root)), {"old.txt", "new.txt"})
            with self.assertRaises(ValueError):
                ci.changed_paths("0" * 40, head, root)


if __name__ == "__main__":
    unittest.main()
