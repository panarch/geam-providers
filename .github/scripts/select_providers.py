"""Choose affected providers for CI; use every provider when unsure."""

import json
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[2]
PATH_DEPENDENCY = re.compile(r'\bpath\s*=\s*"([^"]+)"')
PROVIDER_PHASES = {"erlang", "embedding", "standalone", "executable"}


def main():
    try:
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            cwd=ROOT,
        ))
        rows = discover_providers(metadata)
        integrations = discover_integrations(rows)
        selected, cases, coverage_only, reason = select_event(rows, integrations, metadata)
        result = build_matrices(rows, integrations, selected, cases, coverage_only)
    except (KeyError, OSError, subprocess.CalledProcessError, TypeError, ValueError) as error:
        print("CI declaration: {}".format(error), file=sys.stderr)
        raise SystemExit(1) from error
    print("CI selection: {}".format(reason), file=sys.stderr)
    print(json.dumps(result))


def discover_providers(metadata, root=ROOT):
    members = set(metadata["workspace_members"])
    rows = []
    for package in metadata["packages"]:
        package_metadata = package.get("metadata") or {}
        if package["id"] not in members or "provider" not in package_metadata.get("geam", {}):
            continue
        declaration = package["metadata"]["geam"].get("ci", {})
        if not isinstance(declaration, dict):
            raise ValueError(package["name"] + ": CI declaration must be an object")
        provider = package["metadata"]["geam"]["provider"]
        gleam_package = provider.get("gleam-package") if isinstance(provider, dict) else None
        if not isinstance(gleam_package, str) or not gleam_package:
            raise ValueError(package["name"] + ": gleam-package must be a nonempty string")
        allowed = {"fixture-bin", "example-dir", "erlang-oracle", "runners", "script", "script-phases", "cache-workspaces"}
        if set(declaration) - allowed:
            raise ValueError("{}: unknown CI fields {}".format(package["name"], sorted(set(declaration) - allowed)))
        directory = Path(package["manifest_path"]).resolve().parent.relative_to(root.resolve()).as_posix()
        relative_path(directory)
        script = declaration.get("script", "")
        if not isinstance(script, str):
            raise ValueError(directory + ": script must be a string")
        phases = string_list(declaration.get("script-phases", []), "script-phases")
        if not set(phases) <= PROVIDER_PHASES or bool(script) != bool(phases):
            raise ValueError("{}: script and supported script-phases must be declared together".format(directory))
        if script:
            owner_script(root / directory, script)
        oracle = declaration.get("erlang-oracle", False)
        if type(oracle) is not bool or ("erlang" in phases and not oracle):
            raise ValueError("{}: an erlang hook requires erlang-oracle = true".format(directory))
        binary = declaration.get("fixture-bin", package["name"].replace("-", "_") + "_fixture")
        if not isinstance(binary, str) or not re.fullmatch(r"[A-Za-z0-9_-]+", binary):
            raise ValueError("{}: invalid fixture-bin".format(directory))
        example = declaration.get("example-dir", "")
        if not isinstance(example, str):
            raise ValueError(directory + ": example-dir must be a string")
        if example:
            relative_path(example)
        row = {
            "crate": package["name"], "dir": directory,
            "gleam_package": gleam_package,
            "fixture_bin": binary, "example_dir": example, "erlang_oracle": oracle,
            "script": script, "script_phases": phases,
            "runners": runner_list(declaration.get("runners", ["ubuntu-24.04"])),
            "cache_workspaces": "\n".join([
                ". -> target", directory + "/fixtures/embedding -> target",
                *cache_workspaces(declaration.get("cache-workspaces", []), directory, root),
            ]),
        }
        rows.append(row)
    if not rows:
        raise ValueError("no workspace providers declared")
    return sorted(rows, key=lambda row: row["crate"])


def discover_integrations(rows, root=ROOT):
    crates = {row["crate"] for row in rows}
    integrations = []
    directory = root / "integrations"
    for owner in sorted(directory.iterdir()) if directory.exists() else []:
        if not owner.is_dir() or owner.name.startswith("."):
            continue
        relative_path(owner.relative_to(root).as_posix())
        declaration_path = owner / "ci.json"
        declaration = json.loads(declaration_path.read_text(), object_pairs_hook=unique_object)
        fields = {"schema", "script", "providers", "runners", "cache-workspaces"}
        if set(declaration) != fields or type(declaration["schema"]) is not int or declaration["schema"] != 1:
            raise ValueError("{}: expected CI schema 1 and fields {}".format(declaration_path, sorted(fields)))
        owner_script(owner, declaration["script"])
        providers = string_list(declaration["providers"], "providers")
        if set(providers) - crates:
            raise ValueError("{}: unknown providers {}".format(owner, sorted(set(providers) - crates)))
        integrations.append({
            "name": owner.name, "dir": owner.relative_to(root).as_posix(),
            "script": declaration["script"], "providers": providers,
            "runners": runner_list(declaration["runners"]),
            "cache_workspaces": cache_workspaces(declaration["cache-workspaces"], owner.relative_to(root).as_posix(), root),
        })
    return integrations


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate CI declaration field: " + key)
        result[key] = value
    return result


def string_list(values, field):
    if not isinstance(values, list) or not all(isinstance(value, str) and value for value in values):
        raise ValueError(field + " must be a list of nonempty strings")
    if len(values) != len(set(values)):
        raise ValueError(field + " must not contain duplicates")
    return values


def runner_list(values):
    runners = string_list(values, "runners")
    if not runners or not all(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", runner) for runner in runners):
        raise ValueError("declare at least one valid runner label")
    return runners


def relative_path(value):
    if not isinstance(value, str) or not value or any(char in value for char in "\\:\n\r\t"):
        raise ValueError("invalid relative repository path: {!r}".format(value))
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or path.as_posix() != value:
        raise ValueError("expected a normalized relative repository path: " + value)
    return value


def owner_script(owner, script):
    relative_path(script)
    path = owner / script
    path.resolve().relative_to(owner.resolve())
    if not path.is_file() or path.is_symlink() or path.suffix != ".sh":
        raise ValueError("CI script must be an existing regular Bash file: " + str(path))


def cache_workspaces(values, directory, root):
    result = []
    for value in string_list(values, "cache-workspaces"):
        parts = value.split(" -> ")
        if len(parts) != 2:
            raise ValueError("cache workspace must use 'path -> target': " + value)
        workspace, target = map(relative_path, parts)
        path = root / directory / workspace
        path.resolve().relative_to((root / directory).resolve())
        if not (path / "Cargo.toml").is_file():
            raise ValueError("cache workspace must have a Cargo manifest: " + str(path))
        result.append(directory + "/" + value)
    return result


def git(*args, root=ROOT):
    return subprocess.check_output(["git", *args], cwd=root)


def fixture_manifests(root):
    tracked = git("ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", root=root).split(b"\0")
    return [
        path.decode("utf-8", "surrogateescape")
        for path in tracked
        if path.endswith(b"/Cargo.toml")
        and (b"/fixtures/" in path or b"/examples/" in path)
    ]


def provider_dependencies(rows, metadata, root, manifests):
    by_directory = {(root / row["dir"]).resolve(): row["crate"] for row in rows}
    by_crate = {row["crate"]: row for row in rows}
    dependencies = {crate: set() for crate in by_crate}
    members = set(metadata["workspace_members"])

    for package in metadata["packages"]:
        if package["id"] not in members or package["name"] not in by_crate:
            continue
        for dependency in package["dependencies"]:
            path = dependency.get("path")
            target = by_directory.get(Path(path).resolve()) if path else None
            if target and target != package["name"]:
                dependencies[package["name"]].add(target)

    for relative_path in manifests:
        owner = next(
            (row["crate"] for row in rows if relative_path.startswith(row["dir"] + "/")),
            None,
        )
        if owner is None:
            continue
        manifest = root / relative_path
        content = manifest.read_text()
        paths = PATH_DEPENDENCY.findall(content)
        if len(paths) != len(re.findall(r"\bpath\s*=", content)):
            raise ValueError("Unsupported fixture path syntax")
        for path in paths:
            target = by_directory.get((manifest.parent / path).resolve())
            if target and target != owner:
                dependencies[owner].add(target)
    return dependencies


def changed_paths(base, head, root=ROOT):
    if not base or not head or not base.strip("0"):
        raise ValueError("Missing diff endpoint")
    data = git("diff", "--name-only", "-z", "--no-renames", base, head, "--", root=root)
    return [path.decode("utf-8", "surrogateescape") for path in data.split(b"\0") if path]


def manifest_sections(content):
    sections = {"": []}
    current = ""
    for line in content.splitlines():
        match = re.fullmatch(r"\[([A-Za-z0-9_.-]+)\]", line.strip())
        if match:
            current = match.group(1)
            if current in sections:
                raise ValueError("Repeated manifest section")
            sections[current] = []
        elif line.strip():
            sections[current].append(line.strip())
    return sections


def manifest_additions(base, head):
    """Return added workspace members, or None if an existing setting changed."""
    try:
        old = manifest_sections(base)
        new = manifest_sections(head)
        if old.keys() != new.keys():
            return None

        def workspace_members(lines):
            members = [line for line in lines if line.startswith("members")]
            if len(members) != 1:
                raise ValueError("Expected one workspace members list")
            match = re.fullmatch(r"members\s*=\s*(\[.*\])", members[0])
            if not match:
                raise ValueError("Unsupported workspace members format")
            value = json.loads(match.group(1))
            if not isinstance(value, list) or not all(isinstance(member, str) for member in value):
                raise ValueError("Invalid workspace members")
            if len(value) != len(set(value)):
                raise ValueError("Duplicate workspace member")
            return value, [line for line in lines if line != members[0]]

        old_members, old_workspace = workspace_members(old["workspace"])
        new_members, new_workspace = workspace_members(new["workspace"])
        if old_workspace != new_workspace or not set(old_members) <= set(new_members):
            return None

        def dependencies(lines):
            result = {}
            for line in lines:
                match = re.fullmatch(r"([A-Za-z0-9_-]+)\s*=\s*(.+)", line)
                if not match or match.group(1) in result:
                    raise ValueError("Unsupported workspace dependency format")
                result[match.group(1)] = match.group(2)
            return result

        old_dependencies = dependencies(old["workspace.dependencies"])
        new_dependencies = dependencies(new["workspace.dependencies"])
        if any(new_dependencies.get(name) != value for name, value in old_dependencies.items()):
            return None
        if any(
            old[name] != new[name]
            for name in old
            if name not in ("workspace", "workspace.dependencies")
        ):
            return None
        return set(new_members) - set(old_members)
    except (KeyError, TypeError, ValueError):
        return None


def lock_additions_only(base, head):
    """Accept new lock packages only; compare every existing record verbatim."""
    def split(content):
        parts = re.split(r"(?=^\[\[package\]\]\s*$)", content, flags=re.MULTILINE)
        return parts[0].strip(), Counter(part.strip() for part in parts[1:])

    old_header, old_packages = split(base)
    new_header, new_packages = split(head)
    return old_header == new_header and not old_packages - new_packages


def affected_providers(paths, rows, dependencies, added_members=(), manifest_safe=True, lock_safe=True, integration_dirs=()):
    all_crates = {row["crate"] for row in rows}
    if not manifest_safe or not lock_safe:
        return all_crates, "shared Cargo manifest or lockfile changed"

    by_directory = {row["dir"]: row["crate"] for row in rows}
    direct = set()
    integration_changed = False
    for member in added_members:
        if member not in by_directory:
            return all_crates, "new workspace member is not a discovered provider"
        direct.add(by_directory[member])

    for path in paths:
        if any(path.startswith(directory + "/") for directory in integration_dirs):
            integration_changed = True
            continue
        if path in ("Cargo.toml", "Cargo.lock", "README.md") or (
            path.startswith("docs/") and path.endswith(".md")
        ):
            continue
        owner = next(
            (row["crate"] for row in rows if path.startswith(row["dir"] + "/")),
            None,
        )
        if owner is None:
            return all_crates, "shared or unknown path changed"
        direct.add(owner)

    if not direct:
        if integration_changed:
            return set(), "integration-only change"
        return all_crates, "no provider change"

    affected = set(direct)
    while True:
        dependents = {
            crate for crate, requirements in dependencies.items() if requirements & affected
        }
        if dependents <= affected:
            break
        affected |= dependents
    return affected, "affected providers"


def integration_workspace_dependencies(providers, root=ROOT):
    names = set()
    for directory in providers:
        sections = manifest_sections((root / directory / "Cargo.toml").read_text())
        for line in sections["dependencies"]:
            if line.startswith("#") or "workspace" not in line:
                continue
            match = re.fullmatch(
                r"([A-Za-z0-9_-]+)(?:\.workspace\s*=\s*true|\s*=\s*\{.*\bworkspace\s*=\s*true.*\})",
                line,
            )
            if not match:
                raise ValueError("Unsupported workspace dependency syntax")
            names.add(match.group(1))
    return names


def relevant_workspace_manifest_change(before, after, dependency_names):
    def relevant(content):
        sections = manifest_sections(content)
        if "workspace" not in sections or "workspace.dependencies" not in sections:
            raise ValueError("Missing workspace manifest sections")
        sections = {
            section: [line for line in lines if not line.startswith("#")]
            for section, lines in sections.items()
        }
        sections["workspace"] = [
            line for line in sections["workspace"] if not line.startswith("members")
        ]
        dependencies = []
        for line in sections["workspace.dependencies"]:
            match = re.fullmatch(r"([A-Za-z0-9_-]+)\s*=\s*(.+)", line)
            if not match:
                raise ValueError("Unsupported workspace dependency format")
            if match.group(1) in dependency_names:
                dependencies.append(line)
        sections["workspace.dependencies"] = dependencies
        return sections

    return relevant(before) != relevant(after)


def select_event(rows, integrations, metadata, root=ROOT, env=None):
    env = os.environ if env is None else env
    all_crates = {row["crate"] for row in rows}
    all_cases = {case["name"] for case in integrations}
    provider = env.get("COVERAGE_PROVIDER", "")
    runner = env.get("COVERAGE_RUNNER", "")
    if provider or runner:
        if not provider or not runner:
            raise ValueError("Set both coverage_provider and coverage_runner for a focused run")
        if not any(row["crate"] == provider and runner in row["runners"] for row in rows):
            raise ValueError("No declared coverage target for {} on {}".format(provider, runner))
        return {provider}, set(), True, "focused coverage"
    if env.get("CI_EVENT") not in ("push", "pull_request"):
        cases = all_cases if env.get("RUN_INTEGRATIONS") == "true" else set()
        return all_crates, cases, False, "manual run"
    base = env.get("CI_BASE_SHA", "")
    head = env.get("GITHUB_SHA", "")
    try:
        paths = changed_paths(base, head, root)
        added_members = set()
        manifest_safe = True
        lock_safe = True
        root_manifest_before = None
        root_manifest_after = None
        if "Cargo.toml" in paths:
            root_manifest_before = git("show", base + ":Cargo.toml", root=root).decode()
            root_manifest_after = (root / "Cargo.toml").read_text()
            added_members = manifest_additions(
                root_manifest_before,
                root_manifest_after,
            )
            manifest_safe = added_members is not None
        if "Cargo.lock" in paths:
            lock_safe = lock_additions_only(
                git("show", base + ":Cargo.lock", root=root).decode(),
                (root / "Cargo.lock").read_text(),
            )
        dependencies = provider_dependencies(rows, metadata, root, fixture_manifests(root))
        selected, reason = affected_providers(
            paths, rows, dependencies, added_members or (), manifest_safe, lock_safe,
            [case["dir"] for case in integrations],
        )
        cases = affected_integrations(
            paths, rows, integrations, dependencies, selected, reason,
            root_manifest_before, root_manifest_after, lock_safe, root,
        )
    except (KeyError, OSError, subprocess.CalledProcessError, TypeError, UnicodeError, ValueError):
        selected, cases, reason = all_crates, all_cases, "change analysis unavailable"
    return selected, cases, False, reason


def affected_integrations(paths, rows, integrations, dependencies, selected, reason, before, after, lock_safe, root):
    if reason == "shared or unknown path changed" or not lock_safe:
        return {case["name"] for case in integrations}
    directories = {row["crate"]: row["dir"] for row in rows}
    cases = set()
    for case in integrations:
        if any(path.startswith(case["dir"] + "/") for path in paths):
            cases.add(case["name"])
            continue
        requirements = set(case["providers"])
        while True:
            expanded = requirements | set().union(*(dependencies[crate] for crate in requirements))
            if expanded == requirements:
                break
            requirements = expanded
        if reason == "affected providers" and requirements & selected:
            cases.add(case["name"])
            continue
        if "Cargo.toml" in paths:
            names = integration_workspace_dependencies([directories[crate] for crate in requirements], root)
            if relevant_workspace_manifest_change(before, after, names):
                cases.add(case["name"])
    return cases


def build_matrices(rows, integrations, selected, cases, coverage_only, env=None):
    env = os.environ if env is None else env
    providers = [{key: value for key, value in row.items() if key != "runners"} for row in rows]
    targets = []
    for row in rows:
        if row["crate"] not in selected:
            continue
        for runner in row["runners"]:
            if coverage_only and runner != env.get("COVERAGE_RUNNER"):
                continue
            targets.append({**{key: value for key, value in row.items() if key != "runners"}, "runner": runner})
    by_runner = {}
    for case in integrations:
        if case["name"] not in cases:
            continue
        for runner in case["runners"]:
            by_runner.setdefault(runner, []).append(case)
    integration_targets = []
    for runner, runner_cases in sorted(by_runner.items()):
        caches = [". -> target/geam-cli-build"]
        for case in runner_cases:
            caches.extend(case["cache_workspaces"])
        integration_targets.append({
            "runner": runner,
            "cases": [{"dir": case["dir"]} for case in runner_cases],
            "cache_workspaces": "\n".join(dict.fromkeys(caches)),
        })
    return {
        "providers": {"include": providers}, "targets": {"include": targets},
        "has_targets": bool(targets), "coverage_only": coverage_only,
        "integrations": {"include": integration_targets}, "has_integrations": bool(integration_targets),
    }


if __name__ == "__main__":
    main()
