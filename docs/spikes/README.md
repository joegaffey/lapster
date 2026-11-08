# Spike reports

Each spike writes `docs/spikes/<name>.md` on an isolated `spike/<name>`
worktree or branch. The spike agent commits the report and proposed changes
locally and never pushes or triggers deployments. The architect reviews the
report and diff, signs off, and decides `discard`, `test`, `share`, or
`promote`. After sign-off, the architect may push a shared spike branch and
deploy it to the dev preview.

If a promoted spike changes architecture, the architect writes an ADR in
`docs/adr/` as part of the strict implementation. If it leaves debt, the
architect records it in `docs/tech-debt.md`.

## External validation

The spike agent never pushes. After the architect signs off with decision
`test`, the branch can be validated on a separate target machine.

For browser-based validation, use the dev preview:

1. Push the signed-off spike branch to `joegaffey/lapster` as `spike/<name>`.
2. Trigger the dev preview:
   `gh workflow run pages.yml -R joegaffey/lapster-dev -f ref=spike/<name>`
3. Open https://joegaffey.com/lapster-dev/ on the target machine.

For local or hardware tests that need the code on the target machine, use a
dedicated spike remote or a git bundle.

Online, using a dedicated spike remote:

```bash
# Architect, after sign-off
git remote add spike <spike-remote-url>   # once
git push spike spike/<name>

# Rig
git fetch spike spike/<name>:spike/<name>
git switch spike/<name>
```

Offline, using a git bundle:

```bash
# Dev machine
git bundle create spike-<name>.bundle spike/<name>

# Copy the bundle to the rig, then
git fetch ./spike-<name>.bundle spike/<name>:spike/<name>
git switch spike/<name>
```

A test share is not a merge or promotion. Promotion still goes through the
strict workflow.

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

discard | test | share | promote

## Open questions

## Sign-off

Spike sign-off
- Spike: spike/<name>
- Interfaces tested:
- Decision:
- Verification:
- Architect:
```
