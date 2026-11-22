# 0013. AI racing and online contact direction

- Status: accepted
- Date: 2026-11-21
- Deciders: architect, @joegaffey
- Related: ADR-0011, ADR-0008, `GOALS.md`,
  `docs/features/track-race-mvp.md`, `docs/multiplayer.md`

## Context

`GOALS.md` previously listed AI opponents as a non-goal. The product owner has
now approved AI racing and online contact as the next product direction on top
of the race layout foundation from ADR-0011.

The shared foundation exists: a race-ready track pack can carry `RaceData`
with a closed layout, start/finish, grid slots, sectors, and track identity.
The Race Loop fixture and launcher entry prove the contract end to end. What is
missing is the runtime that turns a layout into race state, opponents, and
contact.

The relay multiplayer prototype is intentionally non-authoritative. Clients run
local physics, upload snapshots at 30 Hz, and render remote cars as ghosts. Any
online contact work must preserve that constraint unless a separate
server-authoritative decision is made.

## Decision

1. AI racing and online contact are approved product goals. Remove AI opponents
   from the goals non-goals; keep damage, weather, tire thermal/wear,
   server-authoritative physics, anti-cheat, and competitive ranking out of
   scope.

2. Implement the direction in this order:
   - **Race manager/lap progress first.** Consume `RaceData` to expose lap,
     sector, start/finish, wrong-way, and progress state, with a small HUD.
   - **AI opponents second.** Add a multi-vehicle simulation path and an AI
     controller that use the layout and grid slots.
   - **Online contact third.** Add car-to-car contact and the relay messages
     needed to observe it, keeping the relay non-authoritative.

3. Keep the race manager deterministic and layout-driven. It is the shared
   source of lap and position state for the HUD, AI, and online clients. It
   must not depend on the visual racing line.

4. Keep online contact client-side and relay-based for now. Clients continue to
   compare `id + source_hash + layout_id`; the relay remains a message hub, not
   a physics authority. A future authoritative model requires a separate ADR.

5. Treat new race-state, AI, and contact contracts as architect-owned shared
   contracts. Add mirrors and versioning before worker implementation.

## Consequences

### Positive

- Product direction now matches the implemented race layout foundation.
- Race manager state is reusable by the HUD, AI, and online positions.
- The first batch is visible and testable without a multi-vehicle worker or
  netcode.
- The relay remains simple and consistent with the zero-backend goal.

### Negative / risks

- Scope expands beyond the original solo hotlap and asset-pipeline goals.
- Multi-vehicle simulation and AI will pressure the current single-vehicle
  worker, snapshot, and rendering paths.
- Online contact is the highest-risk batch: interpolation, latency, collision
  ownership, and cheat resistance are all unsolved.
- Non-authoritative contact can look inconsistent between clients.

## Alternatives considered

- **Keep AI opponents as a non-goal.** Rejected by the product owner.
- **Start with AI before race progress.** Rejected: AI and online positions
  both need lap/progress state, and a visible race-manager slice is smaller.
- **Start with online contact.** Rejected: it is the largest and riskiest batch
  and depends on position/contact semantics that do not exist yet.
- **Make the relay authoritative.** Rejected for this phase: it conflicts with
  the zero-backend goal and would require a separate security and protocol
  decision.

## Follow-up

- Add the race manager/lap HUD batch and its contract.
- Add the AI opponent batch, including the multi-vehicle worker and one AI
  controller.
- Add the online contact batch and a separate ADR for the contact protocol.
- Update `PROGRESS.md` and the race feature plan at each batch boundary.
- Add AC `fast_lane.ai` import or runtime racing-line derivation before AI
  line-following quality work.
