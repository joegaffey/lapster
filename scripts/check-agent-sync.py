#!/usr/bin/env python3
"""Check canonical role cards against committed opencode adapters."""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AGENTS_DIR = ROOT / "agents"
OPENCODE_DIR = ROOT / ".opencode" / "agents"


def frontmatter(path):
    lines = path.read_text().splitlines()
    if not lines or lines[0].strip() != "---":
        return []
    try:
        end = lines.index("---", 1)
    except ValueError:
        return []
    return lines[1:end]


def list_field(lines, key):
    values = []
    in_field = False
    for line in lines:
        if re.match(rf"^{re.escape(key)}:\s*$", line):
            in_field = True
            continue
        if not in_field:
            continue
        match = re.match(r"^\s+-\s+(.+?)\s*$", line)
        if match:
            values.append(match.group(1).strip().strip('"'))
            continue
        if line.strip() and not line.startswith(" "):
            in_field = False
    return values


def scalar_field(lines, key):
    for line in lines:
        match = re.match(rf"^{re.escape(key)}:\s*(.+?)\s*$", line)
        if match:
            return match.group(1).strip().strip('"')
    return None


def edit_allow_patterns(lines):
    patterns = []
    in_edit = False
    for line in lines:
        if re.match(r"^\s*edit:\s*$", line):
            in_edit = True
            continue
        if not in_edit:
            continue
        if re.match(r"^\s*[a-z_]+:\s*$", line):
            in_edit = False
            continue
        match = re.match(r'^\s+"(.+?)":\s*allow\s*$', line)
        if match:
            patterns.append(match.group(1))
    return patterns


def check_role(card_path, adapter_path, expect_edit_deny=False):
    errors = []
    if not adapter_path.exists():
        return [f"missing opencode adapter: {adapter_path.relative_to(ROOT)}"]

    card_lines = frontmatter(card_path)
    adapter_lines = frontmatter(adapter_path)

    if expect_edit_deny:
        if '"*": deny' not in "\n".join(adapter_lines):
            errors.append(f"{card_path.stem}: expected edit deny-all in adapter")
        return errors

    allows = edit_allow_patterns(adapter_lines)
    for path in list_field(card_lines, "owned_paths"):
        if path not in allows:
            errors.append(
                f"{card_path.stem}: owned path not allowed in adapter edit: {path}"
            )
    return errors


def check_runtime_policy():
    errors = []

    architect = frontmatter(OPENCODE_DIR / "architect.md")
    if scalar_field(architect, "steps") is not None:
        errors.append("architect: steps must be omitted for build parity")

    for card in sorted((AGENTS_DIR / "domains").glob("*.md")):
        adapter = frontmatter(OPENCODE_DIR / f"{card.stem}.md")
        if scalar_field(adapter, "steps") != "80":
            errors.append(f"{card.stem}: expected steps: 80")

    for name in ("reviewer", "spike"):
        adapter = frontmatter(OPENCODE_DIR / f"{name}.md")
        if scalar_field(adapter, "steps") != "80":
            errors.append(f"{name}: expected steps: 80")

    spike = frontmatter(OPENCODE_DIR / "spike.md")
    if scalar_field(spike, "mode") != "subagent":
        errors.append("spike: expected mode: subagent")

    try:
        config = json.loads((ROOT / "opencode.json").read_text())
    except Exception as error:
        errors.append(f"opencode.json: invalid JSON: {error}")
    else:
        if config.get("default_agent") != "architect":
            errors.append("opencode.json: expected default_agent architect")

    return errors


def main():
    errors = []
    errors += check_role(AGENTS_DIR / "architect.md", OPENCODE_DIR / "architect.md")

    for card in sorted((AGENTS_DIR / "domains").glob("*.md")):
        errors += check_role(card, OPENCODE_DIR / f"{card.stem}.md")

    errors += check_role(
        AGENTS_DIR / "reviewer.md",
        OPENCODE_DIR / "reviewer.md",
        expect_edit_deny=True,
    )
    errors += check_role(AGENTS_DIR / "spike.md", OPENCODE_DIR / "spike.md")
    errors += check_runtime_policy()

    if errors:
        print("\n".join(errors))
        return 1

    print("agent sync ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
