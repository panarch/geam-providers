"""Choose Geam provider CI targets, falling back to the full matrix on doubt."""

import json
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PATH_DEPENDENCY = re.compile(r'\bpath\s*=\s*"([^"]+)"')


def git(*args, root=ROOT):
    return subprocess.check_output(["git", *args], cwd=root)


def provider_rows(metadata, root):
    members = set(metadata["workspace_members"])
    rows = []
    directories = set()
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        geam = package.get("metadata", {}).get("geam", {})
        provider = geam.get("provider")
        if provider is None:
            continue
        directory = Path(package["manifest_path"]).parent.resolve().relative_to(root).as_posix()
        ci = geam.get("ci", {})
        runners = ci.get("runners", ["ubuntu-24.04"])
        if (
            not isinstance(runners, list)
            or not runners
            or not all(isinstance(runner, str) and runner for runner in runners)
            or len(runners) != len(set(runners))
        ):
            raise ValueError("Provider runners must be distinct non-empty names")
        if directory in directories:
            raise ValueError("Two providers share a directory")
        directories.add(directory)
        rows.append(
            {
                "crate": package["name"],
                "dir": directory,
                "gleam_package": provider["gleam-package"],
                "fixture_bin": ci.get("fixture-bin", package["name"].replace("-", "_") + "_fixture"),
                "example_dir": ci.get("example-dir", ""),
                "runners": runners,
            }
        )
    if not rows:
        raise ValueError("No workspace providers were discovered")
    return rows


def fixture_manifests(root):
    tracked = git("ls-files", "-z", "--", root=root).split(b"\0")
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
        for path in PATH_DEPENDENCY.findall(manifest.read_text()):
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


def affected_providers(paths, rows, dependencies, added_members=(), manifest_safe=True, lock_safe=True):
    all_crates = {row["crate"] for row in rows}
    if not manifest_safe or not lock_safe:
        return all_crates, "shared Cargo manifest or lockfile changed"

    by_directory = {row["dir"]: row["crate"] for row in rows}
    direct = set()
    for member in added_members:
        if member not in by_directory:
            return all_crates, "new workspace member is not a discovered provider"
        direct.add(by_directory[member])

    for path in paths:
        if path in ("Cargo.toml", "Cargo.lock", "README.md") or path.startswith("docs/"):
            continue
        owner = next(
            (row["crate"] for row in rows if path.startswith(row["dir"] + "/")),
            None,
        )
        if owner is None:
            return all_crates, "shared or unknown path changed"
        direct.add(owner)

    affected = set(direct)
    while True:
        dependents = {
            crate for crate, requirements in dependencies.items() if requirements & affected
        }
        if dependents <= affected:
            break
        affected |= dependents
    return affected, "affected providers" if affected else "documentation only"


def focus_target(rows, crate, runner):
    if not crate and not runner:
        return None
    if not crate or not runner:
        raise ValueError("Set both coverage_provider and coverage_runner for a focused run")
    matches = [row for row in rows if row["crate"] == crate and runner in row["runners"]]
    if len(matches) != 1:
        raise ValueError("No declared coverage target for {} on {}".format(crate, runner))
    return matches[0]


def main():
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            cwd=ROOT,
        )
    )
    rows = provider_rows(metadata, ROOT)
    focused = focus_target(
        rows, os.environ.get("COVERAGE_PROVIDER", ""), os.environ.get("COVERAGE_RUNNER", "")
    )
    coverage_only = focused is not None
    event = os.environ.get("CI_EVENT", "")

    if focused:
        selected = {focused["crate"]}
        reason = "focused coverage"
    elif event in ("push", "pull_request"):
        base = os.environ.get("CI_BASE_SHA", "")
        head = os.environ.get("GITHUB_SHA", "")
        try:
            paths = changed_paths(base, head)
            manifest_safe = True
            lock_safe = True
            added_members = set()
            if "Cargo.toml" in paths:
                added_members = manifest_additions(
                    git("show", base + ":Cargo.toml").decode(),
                    (ROOT / "Cargo.toml").read_text(),
                )
                manifest_safe = added_members is not None
            if "Cargo.lock" in paths:
                lock_safe = lock_additions_only(
                    git("show", base + ":Cargo.lock").decode(),
                    (ROOT / "Cargo.lock").read_text(),
                )
            dependencies = provider_dependencies(rows, metadata, ROOT, fixture_manifests(ROOT))
            selected, reason = affected_providers(
                paths, rows, dependencies, added_members or (), manifest_safe, lock_safe
            )
        except (KeyError, OSError, subprocess.CalledProcessError, TypeError, UnicodeError, ValueError):
            selected, reason = {row["crate"] for row in rows}, "change analysis unavailable"
    else:
        selected = {row["crate"] for row in rows}
        reason = "scheduled or manual full run"

    targets = [
        {**{key: value for key, value in row.items() if key != "runners"}, "runner": runner}
        for row in rows
        if row["crate"] in selected
        for runner in row["runners"]
        if not focused or runner == os.environ["COVERAGE_RUNNER"]
    ]
    values = {
        "providers": {"include": [{key: value for key, value in row.items() if key != "runners"} for row in rows]},
        "targets": {"include": targets},
        "has_targets": "true" if targets else "false",
        "coverage_only": "true" if coverage_only else "false",
    }
    lines = [
        key + "=" + (value if isinstance(value, str) else json.dumps(value, separators=(",", ":")))
        for key, value in values.items()
    ]
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a") as destination:
            destination.write("\n".join(lines) + "\n")
    print("CI selection: {}; {} target(s)".format(reason, len(targets)))
    if not output:
        print("\n".join(lines))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, subprocess.CalledProcessError, ValueError) as error:
        print("::error::{}".format(error), file=sys.stderr)
        sys.exit(1)
