# Brief: drivable multi-layout venue editor

Status: active — canonical segment network (supersedes the loop/shared-point
brief)
Created: 2026-11-08
Amended: 2026-11-10
Owner: @joegaffey
Related: `docs/adr/0006-drivable-venue-network.md`,
`agents/contracts.md`, `agents/protocol.md`, `PLAN.md`, `PROGRESS.md`

## 1. Intent

Author a venue as one continuous drivable road network with several selectable
layouts. Layouts share road sections by referencing canonical segments, and the
car can drive from one layout onto another through junctions. The pit lane is a
drivable path, not a layout.

## 2. Locked decisions

- A venue is a set of junctions, canonical segments, routes, and paths.
- A segment is sampled and baked once. It owns its interior points, endpoint
  tangents, width/profile, kerbs, and props.
- A route is a selectable, closed, ordered list of segment ids plus per-layout
  metadata. It does not own geometry.
- A path is drivable but not selectable. The pit lane is a path.
- The world is the union of every unique segment referenced by a route or path.
- All layouts are drivable simultaneously; `?layout=` selects spawn and
  metadata only.
- Editing a shared segment updates every route that references it.
- **Make unique** clones a segment for one route and repoints that route.
- Snapping composes routes and attaches paths at junctions; it does not keep
  copies in sync.
- Route direction is canonical. Whole-route reversal, per-segment width, and
  asymmetric profiles are deferred.
- Asset schema stays v4. Editor project becomes `lapster/track-project@4`.
- The visual and collision chunks contain the union of unique segments once.
- No per-layout mesh groups, pack swapping, or bundle of per-layout packs.

## 3. Non-goals

- Independent per-layout geometry.
- Geometric overlap detection or CSG.
- Per-segment width and asymmetric profile UI.
- Whole-venue or per-route reversal.
- Pit timing, pit strategy, AI, or multiplayer layout propagation.
- Simultaneous multi-layout timing or race control.

## 4. Project document

`lapster/track-project@4`:

- `settings`: global width/profile and kerb settings.
- `controlPoints`: shared junctions with ids, positions, kinds, and tangents.
- `edges`: canonical segments with id, from/to junction, interior points,
  endpoint tangents, kerbs, prop spans.
- `layouts`: routes with id, name, ordered edge ids, start/finish, spawns,
  sectors, cameras.
- `paths`: drivable non-selectable paths with id, kind, ordered edge ids, pit
  boxes.

The wire names are `controlPoints`, `edges`, `layouts`, and `paths`; the prose
uses junctions, segments, routes, and paths. `@3` projects migrate by turning
consecutive loop point pairs into edges, deduplicating shared ordered pairs, and
creating one route per loop. Re-sampling is accepted. `@1`/`@2` support may
route through the existing parser or be recorded as dropped support. The
canonical Rust document is the venue graph serialized as `@4`; there is no
separate editor model.

## 5. Editor UX

- Route list is the primary UI; select a route to edit.
- Junctions, segments, and shared sections are visible in the viewport.
- Drag a junction: every incident segment and route updates.
- Drag an interior point: only that segment changes.
- Editing a shared segment updates every route that uses it.
- **Make unique** on a shared segment clones it for the selected route.
- Kerb/prop painting targets the selected segment and is shared by every route
  that uses it.
- Add/delete route; add/delete/split segment; clone segment; connect segments
  with snapping.
- Path editing is separate from the route list; pit lane is not a route.
- No variants panel, route ranges, edge list, or shared/unique badges.

## 6. Phases

1. **A** — ADR 0006 and this brief.
2. **B** — `@4` project model, `@3` migration, validation.
3. **C** — canonical segment sampling, route assembly, path assembly; remove
   owner-loop sampling and loop-to-graph conversion.
4. **D** — loop-centric segment/route editor: select route, edit junctions and
   segments, Make unique, paint on segments.
5. **E** — junction pads and cross-layout collision validation.
6. **F** — shared pit lane path and pit boxes.

## 7. Acceptance

- All layouts are drivable in one session without loading a different pack.
- The car can cross a junction from one route onto another.
- A shared segment appears once in the visual and collision chunks.
- Editing a shared segment updates every route that uses it.
- Make unique lets one route diverge without changing the others.
- Kerbs/props on a shared segment appear in every route that uses it.
- Selecting a route sets spawn and metadata; it does not swap geometry.
- The pit lane is drivable and is not listed as a layout.
- No route/edge/variant UI remains.

## 8. Gate

Full gate for code/contract changes; docs-only for the ADR and brief. The
editor Playwright suite is rewritten for `@4`; the old `@3` selectors are not
patched.

## 9. Risks

- Junction pads and collision continuity across layouts.
- `@3` migration changes geometry.
- Segment/route authoring UX clarity.
- Pack size and draw calls with all layouts present in one world.

## 10. Definition of done

`PROGRESS.md` updated; ADR 0006 accepted; project `@4` in sync; continuous
cross-layout driving validated; full gate passes; debt triaged; dev preview
validated for UI changes.
