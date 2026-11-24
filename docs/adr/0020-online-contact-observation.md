# 0020. Non-authoritative online contact observation

- Status: proposed
- Date: 2026-11-23
- Deciders: architect, @joegaffey
- Related: ADR-0013, ADR-0014, ADR-0019, `GOALS.md`,
  `docs/features/online-contact.md`, `docs/multiplayer.md`

## Context

ADR-0013 lists online contact after AI contact. ADR-0019 adds co-simulated
contact for the player and AI in the physics worker. Multiplayer is unchanged:
each client simulates only its own car, uploads 30 Hz snapshots, and renders
other cars as interpolated ghosts with no collision and no server correction.

`GOALS.md` asks that online clients "can observe car-to-car contact". It does
not require server-authoritative contact or remote physical correction. The
relay remains a message hub for the zero-backend goal.

## Decision

1. **Online contact is an observation, not authority.** Each client detects
   overlap between its own car and the interpolated remote ghost frames it
   already renders. On a hit it reports a contact observation to the relay,
   which validates and forwards it to the other room members.

2. **Do not correct remote cars.** A remote ghost stays a ghost; the local car
   keeps its own simulation. No client state is corrected and the relay
   simulates nothing. This is the main scope decision for review.

3. **Add additive `contact` messages to the net protocol.** Add a
   client-to-server and a server-to-client `contact` message. The server stamps
   the sender identity, validates shape and finiteness, checks the target
   player, and rate-limits events. Existing `welcome`, `state`, `roster`,
   `player_left`, `error`, and `pong` messages are unchanged.

4. **Keep contact out of race state.** Observations never change laps, sectors,
   positions, finish order, or results, so disagreeing clients cannot corrupt
   standings.

5. **Validate and rate-limit in both places.** The client debounces per remote
   player and ignores small relative speeds; the relay enforces shape, target,
   and per-sender limits. This reduces noise but is not cheat-resistant,
   consistent with the prototype.

6. **No SAB or snapshot change.** The 64-float snapshot stays the remote pose
   source. Contact observations are separate messages.

## Consequences

### Positive

- Online clients can see and hear contact without making the relay
  authoritative or duplicating remote physics.
- The change is additive at the contract layer; existing packs, snapshots, and
  race state keep working.
- Detection cost is small: up to three remote cars per client.

### Negative / risks

- Online contact is cosmetic: ghosts can still overlap, and two clients may
  observe or time a contact differently.
- Web-side detection samples 30 Hz snapshots interpolated to the render
  cadence, so fast grazes can be missed and slow overlaps can repeat.
- A relay that has not been updated rejects the new client message, so the
  relay and client must be deployed together.
- Clients can spam or forge observations; validation and rate limits reduce
  noise only.

## Alternatives considered

- **Make the relay authoritative for contact.** Rejected by ADR-0013: it
  conflicts with the zero-backend goal and needs a separate security and
  physics-authority decision.
- **Simulate remote cars locally in each worker.** Deferred: it needs remote
  proxy input, worker-side interpolation, and a divergence policy, and it
  doubles the simulated field per client. A follow-up ADR is required if the
  product wants physical online contact.
- **Observation-only with no feedback.** Rejected: a contact event must at
  least render and sound to be observable.
- **Reuse the snapshot wheel-contact flags.** Rejected: those flags describe
  terrain wheel contact, not car-to-car contact.

## Follow-up

- Confirm whether observation-only online contact is acceptable, or require a
  follow-up ADR for kinematic remote proxies and local physical response.
- Implement 3B in `docs/features/online-contact.md` after 3A is accepted.
- Update `docs/multiplayer.md` with the contact message and the limitation.
- Record measured detection cost with three remote ghosts.
