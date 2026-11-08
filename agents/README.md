# agents/ — canonical multi-agent layer

This directory is the tool-neutral source of truth for the workflow.
`AGENTS.md` is the entrypoint; this directory holds the details.

## Layout

- `architect.md` — architect role card.
- `reviewer.md` — optional read-only reviewer role card.
- `spike.md` — optional isolated spike agent role card.
- `protocol.md` — core task packet, manifest, lifecycle, gate, escalation.
- `review-protocol.md` — optional reviewer manifest and rules.
- `spike-protocol.md` — optional spike report, handoff, and sign-off.
- `contracts.md` — contract registry, ownership, mirror rules.
- `domains/<domain>.md` — one role card per worker.

## Using this with any assistant

Point the assistant at `AGENTS.md`, then `agents/protocol.md` and the relevant
role card. Load `agents/review-protocol.md` or `agents/spike-protocol.md` only
when using the optional reviewer or spike. If the assistant supports subagents,
use the role cards as subagent prompts. Do not commit assistant-specific adapter
files unless that assistant is the declared primary (currently opencode).

Portable pattern and setup prompt:
https://gist.github.com/joegaffey/27366c537daf1b9e5799e2c71bdac4f7

## Read matrix

| Role | Read |
|---|---|
| Architect | `AGENTS.md`, `GOALS.md`, `agents/architect.md`, `agents/protocol.md`, `agents/contracts.md`, the current section of `PROGRESS.md`; ADRs and tech debt as needed |
| Worker | `AGENTS.md`, `agents/protocol.md`, `agents/contracts.md`, its `agents/domains/<domain>.md`, and the task packet |
| Reviewer | `AGENTS.md`, `agents/review-protocol.md`, `agents/contracts.md`, relevant domain cards, and the diff |
| Spike | `AGENTS.md` spike section, `agents/spike.md`, `agents/spike-protocol.md`, `agents/contracts.md`, and `docs/spikes/` |

Do not read `SPEC.md`, `PLAN.md`, `README.md`, or archived history in full
unless the task needs them. Target: worker read set under ~5k tokens; architect
core under ~8k.

## Optional reviewer

`reviewer` is read-only. The architect may dispatch it after workers and before
commit for high-risk batches. It returns findings by severity; the architect
decides. It does not replace the architect gate, CI, or human approval.

## Optional spike agent

`spike` is an isolated `mode: all` agent for prototype and architecture-spike
work. It may test interface changes in a `spike/<name>` worktree or branch,
writes `docs/spikes/<name>.md`, commits locally, and never pushes. The
architect reviews the report and diff, signs off, and decides
discard/share/promote. See `docs/spikes/README.md`.

## Agent sync check

Run `python3 scripts/check-agent-sync.py` after changing a role card or an
opencode adapter. CI runs the same check.

## Adding a domain

1. Add a role card in `domains/<domain>.md` with `owned_paths`, `read_paths`,
   `verify`, and `forbid`.
2. Add the worker and verification command to `AGENTS.md`.
3. Add a contract entry if the domain exposes or consumes a new shared
   interface.
4. Add a thin opencode wrapper in `.opencode/agents/<domain>.md` with matching
   permissions.
5. Run the domain's verify command.

## Editing rules

- Canonical role cards are authoritative.
- Opencode wrappers are thin; keep their permissions in sync with the role
  card.
- Only the architect edits contract files and mirrors.
