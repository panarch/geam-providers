"""Check repository documents before and after providers are added."""

from pathlib import Path
import re
import subprocess
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"!?\[[^]]*\]\(([^)]+)\)")


def main() -> int:
    license_file = ROOT / "LICENSE"
    if not license_file.is_file() or "Apache License" not in license_file.read_text():
        print("Missing Apache license at repository root")
        return 1

    tracked = subprocess.check_output(
        ["git", "ls-files", "-z", "--", "*.md"], cwd=ROOT
    ).split(b"\0")
    missing = []
    for name in filter(None, tracked):
        document = ROOT / name.decode()
        for match in LINK.finditer(document.read_text()):
            target = match.group(1).strip()
            if target.startswith("<"):
                target = target.split(">", 1)[0][1:]
            else:
                target = target.split(" ", 1)[0]
            target = unquote(target)
            if urlsplit(target).scheme or target.startswith(("#", "//")):
                continue
            path = target.split("#", 1)[0].split("?", 1)[0]
            if path and not (document.parent / path).exists():
                missing.append(f"{document.relative_to(ROOT)}: {target}")

    for link in missing:
        print(f"Missing local link: {link}")
    if missing:
        return 1

    print("Root license and tracked Markdown links are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
