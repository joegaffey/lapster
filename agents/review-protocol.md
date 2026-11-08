# Reviewer protocol

The reviewer is an optional read-only subagent. It reviews a batch diff against
the contract registry, acceptance criteria, and gate results. Findings are
advisory; the architect decides. Blocking findings go back to the owning
worker.

## Review manifest

The reviewer returns only this JSON object:

```json
{
  "status": "APPROVE | REQUEST_CHANGES | BLOCKED",
  "scope": "batch | <domain>",
  "findings": [
    {
      "severity": "blocking | major | minor | nit",
      "file": "<path>",
      "line": 123,
      "issue": "<...>",
      "suggestion": "<...>"
    }
  ],
  "contracts_checked": ["<path>"],
  "tests_checked": ["<command>"],
  "summary": "<1-2 sentences>"
}
```

## Rules

- The reviewer never edits, commits, or pushes.
- Report findings by severity with file:line references.
- Flag contract drift, security issues, missing tests, cross-domain breakage,
  new technical debt, and spec violations.
- `APPROVE` is advisory, not a GitHub approval.
- Blocking findings go back to the owning worker as a focused task packet.
