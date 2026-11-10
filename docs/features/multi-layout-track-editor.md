# Brief: multi-loop venue editor

Status: amended — loop/shared-control-point model (supersedes the venue-graph
brief)
Created: 2026-11-08
Amended: 2026-11-09
Owner: @joegaffey
Related: `docs/adr/0005-loop-shared-control-points.md`,
`agents/contracts.md`, `agents/protocol.md`, `PLAN.md`, `PROGRESS.md`

## 1. Intent

Author a venue with one or more closed loops that share control points and
therefore share spline pieces. The user selects a loop, moves control points,
paints kerbs/props on segments, and test-drives that loop.

## 2. Locked decisions

- A venue is a set of loops.
- A loop is a closed ordered list of control-point ids.
- Control points are shared objects; moving one updates every loop that
  references it.
- A segment is an ordered consecutive pair `(A -> B)` in a loop.
- A shared segment is a piece of center spline used by multiple loops,
  operationalized as the piece between consecutive shared control points in the
  same direction.
- If a shared stretch starts or ends mid-segment, insert a control point there.
- Kerb/prop spans belong to a segment and are shared when the segment is shared.
- One global width/profile for now; per-control-point width later.
- Start/finish is a control-point kind, shared or unique.
- One shared pit lane later, as an open path between pit-entry/pit-exit points.
- Control points are handles; the final centerline is best-effort.
- Each control point has one shared tangent (explicit or averaged across
  incident loops). Shared pieces are sampled once.
- Asset schema stays v4; editor project becomes `lapster/track-project@3`.
- Remove the variants/edges/routes UI.

## 3. Non-goals

- Geometric overlap detection or CSG for shared pieces.
- Per-loop decoration overrides on a shared segment.
- Variable/asymmetric width UI.
- Pit timing, pit strategy, AI, multiplayer layout sync.

## 4. Project document

`lapster/track-project@3`:

- `settings`: global width/profile and kerb settings.
- `controlPoints`: shared handles with id, position, kind, optional tangent.
- `loops`: id, name, ordered control-point ids, sectors/cameras/spawns.
- `segments`: derived pairs with manual kerb spans and prop spans.
- `pitLane`: optional later.

`@2` projects migrate by turning each route into a loop of control-point ids.

## 4a. Branch workflow

- Select a point `J` on a loop.
- Create a branch point `P1` perpendicular to the loop.
- Append branch points at the open end.
- Select a point `K` on the original loop to close.
- New loop: `J -> P1 -> ... -> Pn -> K -> original points after K back to J`.
- The original loop is unchanged; the branch replaces the `J -> K` arc.
- Branch direction selects the shared original arc.
- Perpendicular is only the initial seed; branch tangents follow the edited
  branch geometry.
- Branch edges use role `branch`; main edges keep the main-loop tangent.

## 5. Editor UX

- Loop list replaces the layout/variant list.
- Select a loop; shared points are visually distinct.
- Drag a shared point -> all dependent loops update.
- **Make unique** copies a shared point for the selected loop only.
- **Duplicate loop** shares all points initially; Make unique to diverge.
- Add/delete points on the selected loop.
- Kerb/prop painting targets the selected segment; shared segment means shared
  decoration.
- No variants panel, route ranges, edge list, or shared badges.

Deletion starts local: remove from the selected loop only, keep the point if
another loop uses it, refuse if the loop drops below three points, prune
unreferenced points. Experiment before adding global delete.

## 6. Phases

1. **A** — ADR 0005 and this brief.
2. **B** — `@3` project model, migration, shared-point/segment validation.
3. **C** — shared segment sampling, averaged tangents, unique-piece geometry,
   shared kerbs/props, per-loop metadata.
4. **D** — loop/shared-point editor: select loop, drag shared points, Make
   unique, duplicate/add/delete points, paint on segments.
5. **E** — shared pit lane.

## 7. Acceptance

- Two loops share control points; moving a shared point moves both.
- A shared segment is baked once and appears identical in both loops.
- Make unique on a shared point lets one loop diverge.
- Kerbs/props on a shared segment update both loops.
- Selecting a loop and test-driving it works.
- No route/edge/variant UI remains.

## 8. Gate

Full gate for code/contract changes; docs-only for the amendment itself.

## 9. Risks

- Shared tangents/best-effort geometry.
- Duplicate/make-unique UX clarity.
- `@2` migration.
- Shared segment identity and boundary insertion.

## 10. Definition of done

`PROGRESS.md` updated; ADR 0005 accepted; project `@3` in sync; loop/shared-point
acceptance met; full gate passes; debt triaged; dev preview validated for UI
changes.
