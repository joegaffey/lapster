# Spike reports

Each spike writes `docs/spikes/<name>.md` on an isolated `spike/<name>`
worktree or branch. The spike agent commits the report and proposed changes
locally and never pushes. The architect reviews the report and diff, signs off,
and decides `discard`, `share`, or `promote`.

If a promoted spike changes architecture, the architect writes an ADR in
`docs/adr/` as part of the strict implementation. If it leaves debt, the
architect records it in `docs/tech-debt.md`.

## Template

```markdown
# Spike: <name>

## Objective

## Interfaces tested

## Proposed changes

- Branch: spike/<name>
- Commits:
- Files:
- Summary:

## Verification

| Command | Passed |
|---|---|
|  |  |

## Findings

## Risks

## Debt

- New debt introduced by the spike:

## Recommendation

discard | share | promote

## Open questions

## Sign-off

Spike sign-off
- Spike: spike/<name>
- Interfaces tested:
- Decision:
- Verification:
- Architect:
```
