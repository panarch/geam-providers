"""Contracts for CI discovery, ownership and affected-target selection."""

import json
import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

import select_providers as ci


ROWS = [
    {"crate": "geam-filepath", "dir": "filepath", "runners": ["ubuntu-24.04"]},
    {"crate": "geam-simplifile", "dir": "simplifile", "runners": ["ubuntu-24.04", "macos-15", "windows-2025"]},
    {"crate": "geam-platform", "dir": "platform", "runners": ["ubuntu-24.04"]},
]
DEPENDENCIES = {"geam-filepath": set(), "geam-simplifile": {"geam-filepath"}, "geam-platform": set()}
ALL = set(DEPENDENCIES)


class SelectionTests(unittest.TestCase):
    def test_changed_owner_and_reverse_dependents(self):
        for directory, expected in (
            ("platform", {"geam-platform"}),
            ("simplifile", {"geam-simplifile"}),
            ("filepath", {"geam-filepath", "geam-simplifile"}),
        ):
            with self.subTest(directory=directory):
                selected, reason = ci.affected_providers([directory + "/src/lib.rs"], ROWS, DEPENDENCIES)
                self.assertEqual(selected, expected)
                self.assertEqual(reason, "affected providers")

    def test_integration_only_and_combined_changes(self):
        for paths, expected in (
            (["integrations/tool/ci.sh"], set()),
            (["integrations/tool/ci.sh", "filepath/src/lib.rs"], {"geam-filepath", "geam-simplifile"}),
        ):
            selected, _ = ci.affected_providers(paths, ROWS, DEPENDENCIES, integration_dirs=["integrations/tool"])
            self.assertEqual(selected, expected)

    def test_shared_unknown_and_removed_integration_paths(self):
        for path in (".github/workflows/ci.yml", ".github/scripts/ci-lib.sh", "LICENSE", "integrations/removed/ci.sh"):
            selected, reason = ci.affected_providers([path], ROWS, DEPENDENCIES)
            self.assertEqual(selected, ALL)
            self.assertEqual(reason, "shared or unknown path changed")

    def test_docs_and_empty_diff_keep_full_provider_policy(self):
        for paths in ([], ["README.md", "docs/development/testing.md"]):
            selected, reason = ci.affected_providers(paths, ROWS, DEPENDENCIES)
            self.assertEqual(selected, ALL)
            self.assertEqual(reason, "no provider change")

    def test_new_member_is_scoped_but_shared_cargo_is_not(self):
        selected, _ = ci.affected_providers(
            ["Cargo.toml", "Cargo.lock", "platform/Cargo.toml", "README.md"],
            ROWS, DEPENDENCIES, added_members={"platform"},
        )
        self.assertEqual(selected, {"geam-platform"})
        for options in ({"added_members": {"xtask"}}, {"manifest_safe": False}, {"lock_safe": False}):
            selected, _ = ci.affected_providers(["Cargo.toml", "Cargo.lock"], ROWS, DEPENDENCIES, **options)
            self.assertEqual(selected, ALL)


class CargoChangeTests(unittest.TestCase):
    MANIFEST = '[workspace]\nmembers = ["filepath"]\nresolver = "3"\n\n[workspace.dependencies]\ngeam = { git = "https://example.test/geam", rev = "old" }\n'
    LOCK = 'version = 4\n\n[[package]]\nname = "geam-filepath"\nversion = "0.1.0"\n\n[[package]]\nname = "geam"\nversion = "0.1.0"\n'

    def test_pure_member_and_dependency_additions(self):
        after = self.MANIFEST.replace('["filepath"]', '["filepath", "platform"]') + 'regex = "1.12"\n'
        self.assertEqual(ci.manifest_additions(self.MANIFEST, after), {"platform"})
        self.assertIsNone(ci.manifest_additions(self.MANIFEST, after.replace('"old"', '"new"')))
        self.assertIsNone(ci.manifest_additions(self.MANIFEST, self.MANIFEST.replace('"filepath"', '"other"')))

    def test_existing_lock_records_must_be_identical(self):
        after = self.LOCK + '\n[[package]]\nname = "geam-platform"\nversion = "0.1.0"\n'
        self.assertTrue(ci.lock_additions_only(self.LOCK, after))
        self.assertFalse(ci.lock_additions_only(self.LOCK, after.replace('name = "geam"', 'name = "core"')))
        self.assertFalse(ci.lock_additions_only(self.LOCK, self.LOCK.replace('"0.1.0"', '"0.2.0"')))

    def test_relevant_shared_inputs(self):
        before = self.MANIFEST + 'regex = "1.12"\nreqwest = "0.13"\n'
        for after in (
            before.replace('["filepath"]', '["filepath", "other"]'),
            before.replace('reqwest = "0.13"', 'reqwest = "0.14"'),
        ):
            self.assertFalse(ci.relevant_workspace_manifest_change(before, after, {"geam", "regex"}))
        for after in (
            before.replace('rev = "old"', 'rev = "new"'),
            before.replace('regex = "1.12"', 'regex = "1.13"'),
            before.replace('resolver = "3"', 'resolver = "2"'),
        ):
            self.assertTrue(ci.relevant_workspace_manifest_change(before, after, {"geam", "regex"}))

    def test_unknown_syntax_cannot_hide_pin_change(self):
        before = self.MANIFEST.replace(
            'geam = { git = "https://example.test/geam", rev = "old" }',
            'geam = {\n git = "https://example.test/geam",\n rev = "old"\n}',
        )
        with self.assertRaises(ValueError):
            ci.relevant_workspace_manifest_change(before, before.replace('"old"', '"new"'), {"geam"})


class FixtureRepository:
    """Independent Git/Cargo workspace; only metadata is run, never a build."""

    def __init__(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="ci-selection 한글 ")
        self.root = Path(self.temporary.name).resolve()
        self.members = ["support"]
        support = self.root / "support"
        (support / "src").mkdir(parents=True)
        (support / "src/lib.rs").write_text("")
        (support / "Cargo.toml").write_text('[package]\nname = "ci-support"\nversion = "0.1.0"\n')
        scripts = self.root / ".github/scripts"
        scripts.mkdir(parents=True)
        shutil.copyfile(ci.ROOT / ".github/scripts/select_providers.py", scripts / "select_providers.py")
        (self.root / ".gitignore").write_text("target/\n")
        self.write_provider("filepath")
        self.write_provider("simplifile", dependency="filepath", declaration={
            "runners": ["ubuntu-24.04", "macos-15", "windows-2025"],
        })
        self.write_provider("platform")
        self.write_integration("paths", ["geam-simplifile"])
        self.write_integration("system", ["geam-platform"], ["ubuntu-24.04"])
        self.git("init", "-q")
        self.git("config", "user.name", "CI fixture")
        self.git("config", "user.email", "ci@example.test")
        self.base = self.commit("base")

    def close(self):
        self.temporary.cleanup()

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, text=True, stderr=subprocess.PIPE).strip()

    def write_provider(self, directory, dependency=None, declaration=None):
        if directory not in self.members:
            self.members.append(directory)
        owner = self.root / directory
        (owner / "src").mkdir(parents=True, exist_ok=True)
        (owner / "src/lib.rs").write_text("")
        (owner / "fixtures/embedding").mkdir(parents=True, exist_ok=True)
        (owner / "fixtures/embedding/Cargo.toml").write_text("[workspace]\n")
        declaration = declaration or {}
        metadata = "".join(key + " = " + json.dumps(value) + "\n" for key, value in declaration.items())
        extra = 'other = { package = "geam-' + dependency + '", path = "../' + dependency + '" }\n' if dependency else ""
        (owner / "Cargo.toml").write_text(
            '[package]\nname = "geam-' + directory + '"\nversion = "0.1.0"\n'
            '[package.metadata.geam.provider]\ngleam-package = "' + directory + '"\n'
            '[package.metadata.geam.ci]\n' + metadata +
            '[dependencies]\ngeam.workspace = true\n' + extra
        )
        if declaration.get("script"):
            script = owner / declaration["script"]
            script.parent.mkdir(parents=True, exist_ok=True)
            script.write_text("#!/usr/bin/env bash\nexit 0\n")
        (self.root / "Cargo.toml").write_text(
            '[workspace]\nmembers = ' + json.dumps(self.members) + '\nresolver = "3"\n\n'
            '[workspace.dependencies]\ngeam = { package = "ci-support", path = "support" }\n'
        )

    def write_integration(self, name, providers, runners=None):
        owner = self.root / "integrations" / name
        owner.mkdir(parents=True, exist_ok=True)
        (owner / "ci.sh").write_text("#!/usr/bin/env bash\nexit 0\n")
        (owner / "ci.json").write_text(json.dumps({
            "schema": 1, "script": "ci.sh", "providers": providers,
            "runners": runners or ["ubuntu-24.04", "macos-15", "windows-2025"],
            "cache-workspaces": [],
        }))

    def commit(self, message):
        subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=self.root, check=True, capture_output=True)
        self.git("add", ".")
        self.git("-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def declarations(self):
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"], cwd=self.root,
        ))
        rows = ci.discover_providers(metadata, self.root)
        return rows, ci.discover_integrations(rows, self.root), metadata

    def select(self, **env):
        rows, cases, metadata = self.declarations()
        return ci.select_event(rows, cases, metadata, self.root, {
            "CI_EVENT": "pull_request", "CI_BASE_SHA": self.base,
            "GITHUB_SHA": self.git("rev-parse", "HEAD"), **env,
        })

    def workflow(self, **env):
        content = (ci.ROOT / ".github/workflows/ci.yml").read_text()
        step = content.split("      - name: Build provider matrix from Cargo metadata\n", 1)[1]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1].split("\n  quality:", 1)[0])
        output = self.root / "workflow-output"
        result = subprocess.run(
            ["bash", "-e", "-o", "pipefail", "-c", script], cwd=self.root, text=True, capture_output=True,
            env={**os.environ, "GITHUB_OUTPUT": str(output), "CARGO_NET_OFFLINE": "true",
                 "CI_EVENT": "workflow_dispatch", "COVERAGE_PROVIDER": "", "COVERAGE_RUNNER": "",
                 "RUN_INTEGRATIONS": "false", **env},
        )
        values = dict(line.split("=", 1) for line in output.read_text().splitlines()) if output.exists() else {}
        output.unlink(missing_ok=True)
        return result, values


class RepositorySelectionTests(unittest.TestCase):
    def setUp(self):
        self.fixture = FixtureRepository()
        self.addCleanup(self.fixture.close)

    def test_new_integration_needs_no_shared_ci_edit(self):
        self.fixture.write_integration("new-cli", ["geam-platform"], ["windows-2025"])
        self.fixture.commit("add integration")
        providers, cases, coverage, reason = self.fixture.select()
        self.assertEqual((providers, cases, coverage, reason), (set(), {"new-cli"}, False, "integration-only change"))
        rows, integrations, _ = self.fixture.declarations()
        matrix = ci.build_matrices(rows, integrations, providers, cases, coverage)
        self.assertEqual(matrix["integrations"]["include"], [{
            "runner": "windows-2025", "cases": [{"dir": "integrations/new-cli"}],
            "cache_workspaces": ". -> target/geam-cli-build",
        }])

    def test_new_provider_with_four_server_hooks_is_scoped(self):
        self.fixture.write_provider("server", "filepath", {
            "script": "fixtures/ci.sh", "script-phases": ["erlang", "embedding", "standalone", "executable"],
            "erlang-oracle": True, "runners": ["windows-2025"],
        })
        head = self.fixture.commit("add server")
        providers, cases, _, reason = self.fixture.select()
        self.assertEqual((providers, cases, reason), ({"geam-server"}, set(), "affected providers"))
        result, values = self.fixture.workflow(CI_EVENT="pull_request", CI_BASE_SHA=self.fixture.base, GITHUB_SHA=head)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(values["has_integrations"], "false")
        self.assertEqual([
            (row["crate"], row["runner"]) for row in json.loads(values["targets"])["include"]
        ], [("geam-server", "windows-2025")])

    def test_hook_edit_runs_owner_and_integration_consumers(self):
        self.fixture.write_provider("platform", declaration={"script": "fixtures/ci.sh", "script-phases": ["standalone"]})
        self.fixture.base = self.fixture.commit("register hook")
        (self.fixture.root / "platform/fixtures/ci.sh").write_text("exit 1\n")
        self.fixture.commit("change hook")
        providers, cases, _, _ = self.fixture.select()
        self.assertEqual((providers, cases), ({"geam-platform"}, {"system"}))

    def test_dependency_reaches_transitive_integration_consumer(self):
        (self.fixture.root / "filepath/src/lib.rs").write_text("// change\n")
        self.fixture.commit("dependency")
        providers, cases, _, _ = self.fixture.select()
        self.assertEqual((providers, cases), ({"geam-filepath", "geam-simplifile"}, {"paths"}))

    def test_rename_selects_both_registered_owners(self):
        (self.fixture.root / "integrations/paths/scenario.txt").write_text("fixture")
        self.fixture.base = self.fixture.commit("scenario")
        self.fixture.git("mv", "integrations/paths/scenario.txt", "integrations/system/scenario.txt")
        head = self.fixture.commit("rename")
        self.assertEqual(set(ci.changed_paths(self.fixture.base, head, self.fixture.root)), {
            "integrations/paths/scenario.txt", "integrations/system/scenario.txt",
        })
        providers, cases, _, _ = self.fixture.select()
        self.assertEqual((providers, cases), (set(), {"paths", "system"}))

    def test_shared_execution_and_cargo_changes_run_all(self):
        for path in (".github/scripts/ci-lib.sh", "Cargo.toml"):
            with self.subTest(path=path):
                original = self.fixture.root / path
                if path == "Cargo.toml":
                    original.write_text(original.read_text().replace('resolver = "3"', 'resolver = "2"'))
                else:
                    original.write_text("# shared execution changed\n")
                self.fixture.commit("shared")
                providers, cases, _, _ = self.fixture.select()
                self.assertEqual((providers, cases), (ALL, {"paths", "system"}))

    def test_missing_diff_is_full_fallback(self):
        providers, cases, _, reason = self.fixture.select(CI_BASE_SHA="missing")
        self.assertEqual((providers, cases, reason), (ALL, {"paths", "system"}, "change analysis unavailable"))

    def test_actual_workflow_manual_and_focused_requests(self):
        for enabled in ("true", "false"):
            result, values = self.fixture.workflow(RUN_INTEGRATIONS=enabled)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((values["has_targets"], values["has_integrations"]), ("true", enabled))
        result, values = self.fixture.workflow(
            COVERAGE_PROVIDER="geam-simplifile", COVERAGE_RUNNER="windows-2025", RUN_INTEGRATIONS="true",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((values["coverage_only"], values["has_integrations"]), ("true", "false"))
        rows = json.loads(values["targets"])["include"]
        self.assertEqual([(row["crate"], row["runner"]) for row in rows], [("geam-simplifile", "windows-2025")])

    def test_actual_workflow_new_integration_is_scoped(self):
        self.fixture.write_integration("third", [])
        head = self.fixture.commit("third integration")
        result, values = self.fixture.workflow(CI_EVENT="pull_request", CI_BASE_SHA=self.fixture.base, GITHUB_SHA=head)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((values["has_targets"], values["has_integrations"]), ("false", "true"))
        self.assertTrue(all(
            row["cases"] == [{"dir": "integrations/third"}]
            for row in json.loads(values["integrations"])["include"]
        ))

    def test_invalid_focused_requests_fail(self):
        for env in ({"COVERAGE_PROVIDER": "geam-filepath"}, {"COVERAGE_PROVIDER": "geam-filepath", "COVERAGE_RUNNER": "missing"}):
            result, _ = self.fixture.workflow(**env)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("coverage", result.stderr)


class DeclarationTests(unittest.TestCase):
    def setUp(self):
        self.fixture = FixtureRepository()
        self.addCleanup(self.fixture.close)

    def test_unregistered_integration_is_an_error(self):
        (self.fixture.root / "integrations/unregistered").mkdir()
        with self.assertRaises(FileNotFoundError):
            self.fixture.declarations()

    def test_invalid_provider_declaration_types_fail_at_the_registration_boundary(self):
        _, _, metadata = self.fixture.declarations()
        geam = next(package["metadata"]["geam"] for package in metadata["packages"] if package["name"] == "geam-platform")
        for value in (False, [], None):
            geam["ci"] = value
            with self.assertRaisesRegex(ValueError, "geam-platform: CI declaration"):
                ci.discover_providers(metadata, self.fixture.root)

    def test_empty_gleam_package_fails_actual_workflow_discovery(self):
        path = self.fixture.root / "platform/Cargo.toml"
        path.write_text(path.read_text().replace('gleam-package = "platform"', 'gleam-package = ""'))
        result, outputs = self.fixture.workflow()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("geam-platform: gleam-package", result.stderr)
        self.assertEqual(outputs, {})

    def test_invalid_integration_declarations_are_not_analysis_fallback(self):
        path = self.fixture.root / "integrations/paths/ci.json"
        original = json.loads(path.read_text())
        for change in (
            {"schema": True}, {"schema": 2}, {"providers": ["missing"]}, {"runners": []},
            {"runners": ["ubuntu-24.04", "ubuntu-24.04"]}, {"script": "../system/ci.sh"},
            {"script": "missing.sh"}, {"cache-workspaces": ["../system -> target"]}, {"unknown": True},
        ):
            with self.subTest(change=change):
                path.write_text(json.dumps({**original, **change}))
                with self.assertRaises(ValueError):
                    self.fixture.declarations()
        path.write_text('{"schema":1,"schema":1}')
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.fixture.declarations()

    def test_invalid_provider_hooks_fail_registration(self):
        for declaration in (
            {"script": "fixtures/ci.sh"}, {"script-phases": ["standalone"]},
            {"script": "fixtures/ci.sh", "script-phases": ["unsupported"]},
            {"script": "fixtures/ci.sh", "script-phases": ["erlang"]},
            {"script": False}, {"example-dir": 0}, {"integration-case": "server"},
        ):
            with self.subTest(declaration=declaration):
                self.fixture.write_provider("platform", declaration=declaration)
                with self.assertRaises(ValueError):
                    self.fixture.declarations()

    def test_symlink_cannot_escape_script_owner(self):
        script = self.fixture.root / "integrations/paths/ci.sh"
        script.unlink()
        script.symlink_to(self.fixture.root / "integrations/system/ci.sh")
        with self.assertRaises(ValueError):
            self.fixture.declarations()

    def test_runner_specific_cache_and_cases(self):
        owner = self.fixture.root / "integrations/paths"
        (owner / "fixtures/embedding").mkdir(parents=True)
        (owner / "fixtures/embedding/Cargo.toml").write_text("[workspace]\n")
        declaration_path = owner / "ci.json"
        declaration = json.loads(declaration_path.read_text())
        declaration["cache-workspaces"] = ["fixtures/embedding -> target"]
        declaration_path.write_text(json.dumps(declaration))
        rows, cases, _ = self.fixture.declarations()
        matrix = ci.build_matrices(rows, cases, ALL, {"paths", "system"}, False)
        for row in matrix["integrations"]["include"]:
            self.assertIn("integrations/paths/fixtures/embedding -> target", row["cache_workspaces"])
            expected = {"paths", "system"} if row["runner"] == "ubuntu-24.04" else {"paths"}
            self.assertEqual({case["dir"].split("/")[-1] for case in row["cases"]}, expected)


class CurrentRepositoryTests(unittest.TestCase):
    def test_real_dependencies_and_declared_shared_inputs(self):
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"], cwd=ci.ROOT,
        ))
        rows = ci.discover_providers(metadata)
        cases = ci.discover_integrations(rows)
        graph = ci.provider_dependencies(rows, metadata, ci.ROOT, ci.fixture_manifests(ci.ROOT))
        self.assertIn("geam-filepath", graph["geam-simplifile"])
        directories = {row["crate"]: row["dir"] for row in rows}
        expected = {"directories": {"geam"}, "clip": {"geam", "regex", "regex-syntax"}}
        by_name = {case["name"]: case for case in cases}
        for name, expected_names in expected.items():
            case = by_name[name]
            names = ci.integration_workspace_dependencies([directories[name] for name in case["providers"]])
            self.assertEqual(names, expected_names)

    def test_fixture_only_dependency_and_unknown_syntax(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "first/fixtures/gleam/Cargo.toml"
            manifest.parent.mkdir(parents=True)
            (root / "second").mkdir()
            manifest.write_text('other = { path = "../../../second" }\n')
            rows = [{"crate": "first", "dir": "first"}, {"crate": "second", "dir": "second"}]
            metadata = {"workspace_members": ["first-id", "second-id"], "packages": [
                {"id": "first-id", "name": "first", "dependencies": []},
                {"id": "second-id", "name": "second", "dependencies": []},
            ]}
            graph = ci.provider_dependencies(rows, metadata, root, ["first/fixtures/gleam/Cargo.toml"])
            self.assertEqual(graph["first"], {"second"})
            manifest.write_text("other = { path = '../../../second' }\n")
            with self.assertRaises(ValueError):
                ci.provider_dependencies(rows, metadata, root, ["first/fixtures/gleam/Cargo.toml"])


if __name__ == "__main__":
    unittest.main()
