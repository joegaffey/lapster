# Spike protocol

The spike is an optional `mode: all` agent for isolated prototype and
architecture-spike work. The user or the architect may propose a spike.

## Report

The spike writes `docs/spikes/<name>.md` and commits it locally on the spike
branch. The Markdown report is required. The JSON manifest is recommended for
machine-readable handoff:

```json
{
  "spike": "<name>",
  "status": "COMPLETE | BLOCKED",
  "objective": "<...>",
  "interfaces_tested": ["<path>"],
  "changes": {
    "branch": "spike/<name>",
    "commits": ["<sha>"],
    "files": ["<path>"],
    "summary": "<...>"
  },
  "verification": [{ "cmd": "<command>", "passed": true }],
  "findings": ["<...>"],
  "risks": ["<...>"],
  "recommendation": "discard | test | share | promote",
  "open_questions": ["<...>"]
}
```

## Handoff

- The spike works in an isolated `spike/<name>` worktree or branch.
- It may test interface and contract changes there.
- It never pushes or triggers deployments.
- The architect reviews the report and diff, re-runs relevant verification, and
  appends a sign-off to the report.
- `discard`: delete the branch and worktree.
- `test`: the architect authorizes a test share for external validation. For
  browser validation, use the dev preview with `ref=spike/<name>`. For local or
  hardware tests, use a dedicated spike remote or a git bundle. This is not a
  merge or promotion.
- `share`: the architect may push the spike branch after sign-off and trigger
  the dev preview, never to `dev` or `main`.
- `promote`: the architect re-implements through the strict workflow. The spike
  branch is never merged directly.
