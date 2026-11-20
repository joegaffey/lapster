#!/usr/bin/env python3
"""Check track-ed dependency pins, lockfile sync, and third-party notices."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
PKG = ROOT / "web" / "track-ed-src"
PIN_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z][0-9A-Za-z.-]*)?$")
NOTICE_ROW_RE = re.compile(
    r"^\|\s*`([^`]+)`\s*\|\s*([^|]+?)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*$"
)


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(f"missing {path}")
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON in {path}: {exc}")


def main() -> None:
    package = load_json(PKG / "package.json")
    lock = load_json(PKG / "package-lock.json")

    deps = package.get("dependencies", {})
    dev_deps = package.get("devDependencies", {})
    errors: list[str] = []

    for field, values in (("dependency", deps), ("devDependency", dev_deps)):
        for name, version in values.items():
            if not isinstance(version, str) or not PIN_RE.fullmatch(version):
                errors.append(f"{field} {name}@{version!r} is not an exact version pin")

    root_package = lock.get("packages", {}).get("", {})
    for field in ("dependencies", "devDependencies"):
        expected = package.get(field, {})
        actual = root_package.get(field, {})
        if actual != expected:
            errors.append(f"package-lock.json root {field} do not match package.json")

    notices_path = PKG / "THIRD_PARTY_NOTICES.md"
    try:
        notices_text = notices_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        fail(f"missing {notices_path}")

    notices: dict[str, dict[str, str]] = {}
    for line in notices_text.splitlines():
        match = NOTICE_ROW_RE.match(line)
        if not match:
            continue
        name, version, license_name = match.group(1), match.group(2), match.group(3)
        notices[name] = {"version": version, "license": license_name}

    for name, version in deps.items():
        notice = notices.get(name)
        if notice is None:
            errors.append(
                f"runtime dependency {name} is missing from THIRD_PARTY_NOTICES.md"
            )
            continue
        if notice["version"] != version:
            errors.append(
                f"runtime dependency {name} version {version} does not match "
                f"notice version {notice['version']}"
            )
        if not notice["license"]:
            errors.append(
                f"runtime dependency {name} has no license in THIRD_PARTY_NOTICES.md"
            )

    for filename in ("LICENSE", "LICENSE-MIT", "LICENSE-APACHE"):
        path = PKG / filename
        if not path.is_file():
            errors.append(f"missing {path}")

    if errors:
        print(
            f"track-ed dependency check failed with {len(errors)} error(s):",
            file=sys.stderr,
        )
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        raise SystemExit(1)

    print(
        f"OK: {len(deps)} runtime notices ok; "
        f"{len(deps) + len(dev_deps)} pinned dependencies; "
        "lockfile in sync; licenses present"
    )


if __name__ == "__main__":
    main()
