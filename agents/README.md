# agents/ — canonical multi-agent layer

This directory is the tool-neutral source of truth for the workflow.
`AGENTS.md` is the entrypoint; this directory holds the details.

## Layout

- `architect.md` — architect role card.
- `reviewer.md` — optional read-only reviewer role card.
- `spike.md` — optional architect-dispatched, isolated spike subagent role card.
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

## Human role

The human is the technical product manager / product owner, not another worker
and not a passive approver.

- The human owns product intent, success criteria, scope, priorities, tradeoffs,
  risk acceptance and release approval.
- The architect translates product intent into technical options, contracts,
  plans and verification. It does not invent product scope.
- Workers implement approved scope and do not negotiate requirements directly
  with the human.
- Escalate ambiguous requirements, product-priority tradeoffs, breaking contract
  changes and cross-domain conflicts to the human.
- Protected-branch approval is product acceptance as well as technical approval
  unless a separate acceptance step is documented.

## Read matrix

| Role | Read |
|---|---|
| Architect | `AGENTS.md`, `GOALS.md`, `agents/architect.md`, `agents/protocol.md`, `agents/contracts.md`, the current section of `PROGRESS.md`; ADRs and tech debt as needed |
| Worker | `AGENTS.md`, `agents/protocol.md`, `agents/contracts.md`, its `agents/domains/<domain>.md`, and the task packet |
| Reviewer | `AGENTS.md`, `agents/review-protocol.md`, `agents/contracts.md`, relevant domain cards, and the diff |
| Spike | `AGENTS.md` spike section, `agents/spike.md`, `agents/spike-protocol.md`, `agents/contracts.md`, and `docs/spikes/` |
| Editor | `AGENTS.md`, `agents/protocol.md`, `agents/contracts.md`, `agents/domains/editor.md`, the editor consolidation plan/report and ADRs, `web/track-ed-src/AGENTS.md` and `SPEC.md` |

Do not read `SPEC.md`, `PLAN.md`, `README.md`, or archived history in full
unless the task needs them. Target: worker read set under ~5k tokens; architect
core under ~8k.

## Optional reviewer

`reviewer` is read-only. The architect may dispatch it after workers and before
commit for high-risk batches. It returns findings by severity; the architect
decides. It does not replace the architect gate, CI, or human product-owner
approval.

## Runtime policy

- The project default agent is `architect` for the Lapster folder.
- The architect has no step cap.
- The built-in plan agent is overridden to 200 steps so long analysis is not
  truncated; architect remains the default workflow agent.
- Workers, the editor, reviewer and spike use an 80-step cap per user input.
- `spike` is subagent-only and is dispatched by the architect.
- Read permissions are broad and centralised in `opencode.json`; generated,
  vendor, lockfile and secret paths remain denied.
- External directories are allowed by default; SSH, auth and server-secret
  paths remain denied.
- Shell commands use role-scoped allowlists. The architect and editor include a
  common read/inspection set; workers rely on their scoped verification
  commands and the read/grep tools. Unknown commands fall back to `ask` and
  destructive commands remain denied.

## Optional spike agent

`spike` is an architect-dispatched subagent for prototype and architecture-spike
work. It may test interface changes in a `spike/<name>` worktree or branch,
writes `docs/spikes/<name>.md`, commits locally, and never pushes. It is not a
primary agent. The architect reviews the report and diff, signs off, and decides
discard/share/promote. See `docs/spikes/README.md`.

## Agent sync check

Run `python3 scripts/check-agent-sync.py` after changing a role card or an
opencode adapter. CI runs the same check. It validates adapter presence, owned
edit paths, the default agent, step caps and spike mode.

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
