# agents/ — canonical multi-agent layer

This directory is the tool-neutral source of truth for the workflow.
`AGENTS.md` is the entrypoint; this directory holds the details.

## Layout

- `architect.md` — architect role card.
- `reviewer.md` — optional read-only reviewer role card.
- `protocol.md` — task packet, manifest, review manifest, lifecycle,
  escalation.
- `contracts.md` — contract registry, ownership, mirror rules.
- `domains/<domain>.md` — one role card per worker.

## Using this with any assistant

Point the assistant at `AGENTS.md`, then `agents/protocol.md` and the relevant
role card. If the assistant supports subagents, use the role cards as subagent
prompts. Do not commit assistant-specific adapter files unless that assistant
is the declared primary (currently opencode).

Portable pattern and setup prompt:
https://gist.github.com/joegaffey/27366c537daf1b9e5799e2c71bdac4f7

## Optional reviewer

`reviewer` is read-only. The architect may dispatch it after workers and before
commit for high-risk batches. It returns findings by severity; the architect
decides. It does not replace the architect gate, CI, or human approval.

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
