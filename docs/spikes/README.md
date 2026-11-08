# Spike reports

Each spike writes `docs/spikes/<name>.md` on an isolated `spike/<name>`
worktree or branch. The spike agent commits the report and proposed changes
locally and never pushes. The architect reviews the report and diff, signs off,
and decides `discard`, `share`, or `promote`.

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
