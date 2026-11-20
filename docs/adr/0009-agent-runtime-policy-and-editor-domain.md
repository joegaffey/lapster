# 0009. Agent runtime policy, editor domain and spike subagent

- Status: accepted
- Date: 2026-11-20
- Deciders: architect, @joegaffey
- Related: `agents/README.md`, `agents/protocol.md`, `agents/domains/editor.md`,
  `docs/editor-consolidation-report.md`, `docs/features/editor-consolidation-plan.md`

## Context

Log analysis of the Lapster sessions showed that agent step caps and permission
prompts were causing avoidable truncation and repeated approvals:

- the architect used 50 steps and hit the cap;
- the built-in plan agent used 50 steps and hit the cap during a long analysis;
- render subagents used 30 steps and hit the cap in 9 of 20 sessions;
- web subagents hit the 30-step cap in 1 of 2 sessions;
- build had no cap and reached 81 steps in one Lapster session;
- 96 shell `ask` decisions and 131 external-directory `ask` decisions were
  recorded, mostly for read-only inspection, verification and `/tmp` or
  sibling-repository access;
- the project had no default `default_agent`, so build was the de facto primary;
- there was no editor domain even though track-ed is now a distinct ~18k-line
  TypeScript editor with an active consolidation plan;
- `spike` was `mode: all` even though the architect owns spike dispatch and
  sign-off.

The existing adapters also duplicated the same read deny list in every file.

## Decision

- Set the Lapster project default agent to `architect`.
- Remove the architect step cap for build parity.
- Set the editor, all workers, reviewer and spike to 80 steps.
- Make `spike` a subagent-only role, dispatched by the architect. It is never a
  primary session.
- Add an `editor` worker domain covering all web-side editor aspects:
  track-ed, track import, the legacy procedural editor, and editor browser
  tests. The Rust legacy editor module remains coordinated with `render`.
- Centralise the common read policy in `opencode.json` and remove duplicate
  read blocks from agent adapters.
- Allow external directories by default, except SSH, OpenCode auth and server
  secret paths.
- Add a shared read/inspection shell allowlist and role-specific verification
  commands. Unknown shell commands remain `ask`; destructive operations remain
  denied.
- Keep `deepseek-v4.1-flash` as the model for all agents.
- Keep the current V1 `permission`/`bash`/`task` configuration shape for this
  change. Migrating to V2 `permissions`/`shell`/`subagent` is a separate,
  verified follow-up.

## Consequences

### Positive

- The architect is no longer more limited than build.
- Long render/editor tasks are less likely to stop mid-turn.
- Read-only inspection and external repository checks stop generating routine
  permission prompts.
- Editor work has a clear owner and verification commands.
- Spike work stays behind architect dispatch and review.
- The default agent matches the contract-bound workflow.

### Negative / risks

- Unbounded architect steps can increase cost or loop time; the architect gate
  and task packets remain the control.
- Broader shell and external access increases the trust placed in agent
  behaviour. Secrets and generated files remain denied by read rules and
  external-directory exceptions.
- The `editor` domain overlaps `web` at the path level; the web adapter
  explicitly denies editor-owned paths, and task packets must not assign
  overlapping work.
- The V1 permission configuration is deprecated; a V2 migration will be needed
  later and must update `scripts/check-agent-sync.py`.

## Alternatives considered

- **Keep the architect at 50 steps** — rejected; logs show it is the primary
  orchestration role and build already needed more.
- **Make every agent unlimited** — rejected; caps keep worker sessions bounded
  and reduce runaway cost.
- **Keep spike as `mode: all`** — rejected; the architect owns spike dispatch
  and sign-off.
- **Add editor paths to `web` instead of a new domain** — rejected; track-ed is
  a distinct editor stack and the web worker is already broad.
- **Migrate to V2 permissions in the same change** — deferred; it would mix a
  behaviour change with a schema migration and make rollback harder.

## Follow-up

- Restart opencode after these changes so the new default agent, limits and
  permissions load.
- Monitor new sessions for cap hits and permission prompts; adjust the 80-step
  worker cap only if data shows it is needed.
- Add a plan-agent step override if plan sessions continue to hit 50.
- Migrate `permission`/`bash`/`task` to V2 `permissions`/`shell`/`subagent` in a
  separate change, updating the sync check and testing one agent first.
- Keep the editor domain ownership aligned with the consolidation plan as
  legacy editor removal proceeds.
