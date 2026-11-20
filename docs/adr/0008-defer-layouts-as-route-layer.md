# 0008. Defer layouts and treat them as a future route layer over track-ed sections

- Status: accepted; layout implementation is deferred. The route-layer shape below
  is the proposed design if the feature is prioritised; implementation requires
  a separate schema/implementation decision.
- Date: 2026-11-20
- Deciders: architect, @joegaffey
- Related: `docs/editor-consolidation-report.md`,
  `docs/features/editor-consolidation-plan.md`, ADR-0003, ADR-0004, ADR-0007,
  `docs/features/venue-graph-plan.md`

## Context

The legacy `lapster/track-project@6` model has first-class layouts:

```text
layouts: [{
  id, name,
  route: [{ sectionId, entry, exit, wrap? }...],
  startFinish,
  spawns, sectors, cameras
}]
```

A layout is a named route through the venue's sections, with per-layout
start/finish, spawns, sectors and cameras. The legacy bake uses the active
layout for route sampling, start/finish derivation, route-local decoration
slices and exported metadata.

That support is substantially dormant in practice:

- the legacy editor UI loads and edits only the active layout;
- it exposes set-start/finish but has no add/switch/delete layout UI;
- `editor_track_pack()` exports one pack from one active layout;
- Drive has no layout selector, so multiple layouts are not consumable
  end-to-end;
- the current Lapster runtime uses spawn, surfaces and collision rather than
  sectors, track cameras or start/finish.

track-ed currently has no layout concept:

- one `spawn` selecting a centreline and fraction;
- all sections baked into one track;
- track length is the sum of section lengths, not a selected route length;
- racing line is generated per section;
- no start/finish, sectors, route legs or per-layout metadata;
- `cameras` exist in the project schema and manifest export, but there is no
  authoring UI or layout association.

The consolidation decision (ADR-0007) promotes track-ed and treats
multi-layout/routes/start-finish/sectors as deprecated for the current product
scope. This ADR records how layouts are handled: they are not carried into
track-ed now, and if they return they must be a route layer over the existing
section/junction graph rather than a second geometry model.

## Decision

- **Do not implement layouts in track-ed as part of the consolidation.**
  track-ed remains section-first: sections and junctions own geometry; spawn
  selects the driving start.
- **Deprecate the legacy layout model and UI.** Do not port the legacy
  `layouts`, route-leg, start/finish, sectors, per-layout spawns or per-layout
  camera model into track-ed during legacy removal.
- If multi-circuit or multiple-route authoring becomes a product requirement,
  add layouts to track-ed as a **route and metadata layer over existing
  sections**, not as new geometry:
  - sections and junctions remain the only source of road/terrain geometry;
  - a layout selects an ordered, contiguous route through sections;
  - the route uses the same section curves and sampling as the sections; there
    is no separate route spline;
  - all sections may still be baked; the layout selects what is driven and
    what route metadata is exported.
- The proposed future shape is:

```text
layouts: [{
  id,
  name,
  route: [{ sectionId, entry, exit, wrap? }...],
  startFinish,
  spawns,
  sectors,
  cameras
}]
```

- `entry`/`exit` reference a point id or a closed-section seam, matching the
  existing section model.
- Route length is computed from the selected route, not from the sum of all
  section lengths.
- Export continues to produce one pack for one selected layout unless Lapster
  gains a runtime layout selector and the pack/manifest can carry multiple
  layouts.
- Racing line stays per-section initially. A layout-aware racing line is a
  separate future decision, not a reason to delay section-level authoring.
- Decorations stay section-local. Route-local decoration slicing is not
  reintroduced unless a concrete per-layout decoration requirement appears.
- Validation must reject unknown fields, duplicate layout ids, missing section
  references, non-contiguous legs and start/finish points outside the route.
- Introducing layouts requires a new `track-ed/project@N` schema version, new
  contract fixtures and an export-profile review. Existing projects should
  receive a synthesized default layout so they continue to open.
- The legacy `lapster/track-project@6` layout model is not a migration target;
  legacy project migration remains a separate decision under ADR-0007.

## Consequences

### Positive

- No dormant layout complexity is added to the editor during consolidation.
- track-ed's kernel stays focused on section/junction geometry and the baked
  surface contract.
- The current product keeps one clear meaning for spawn and track length.
- If layouts return, they can be added without duplicating road geometry or
  reviving the legacy bake.
- The future model is consistent with track-ed's section-point structure and
  with the existing validation/serialization style.

### Negative / risks

- Multi-circuit/multi-route authoring is unavailable until the feature is
  prioritised.
- Start/finish authoring is lost when the legacy editor is removed.
- Adding layouts later requires a schema bump, contract changes, UI work,
  route-length/export work and tests.
- Users who want multiple circuits must currently duplicate projects or export
  separate variants.
- If Lapster later needs multiple layouts in one pack, runtime and manifest
  support will be required beyond the editor.

## Alternatives considered

- **Port the legacy layout model now** — rejected; most of it is dormant, the
  UI never exposed multi-layout authoring, and it would add contract and
  validation surface before the single-editor consolidation is complete.
- **Add only start/finish now** — deferred; start/finish is part of the
  deprecated layout model and has no confirmed runtime consumer. It can be
  reintroduced as a layout feature or a small spawn/lap-timing feature later.
- **Copy a project per circuit** — accepted as the current workaround; not a
  substitute for a first-class layout model if multi-circuit authoring becomes
  a real requirement.
- **Keep layouts in the legacy editor indefinitely** — rejected by ADR-0007;
  it perpetuates a second authoring path and does not provide an end-to-end
  multi-layout workflow.
- **Implement layouts as separate geometry copies** — rejected; it duplicates
  sections, decoration data and junction truth, and would reintroduce the
  consistency problems the section graph was designed to avoid.

## Follow-up

- Revisit only if multi-circuit/multi-route authoring becomes a product
  requirement.
- Before implementation, write an implementation ADR/spec covering the schema
  version, route validation, export contract, migration/default-layout
  behaviour and UI.
- Decide whether Lapster will consume multiple layouts at runtime or whether
  export selects one layout at a time.
- Add fixtures for multi-loop venues, route legs, start/finish and layout
  switching; extend the fixture matrix and export determinism checks.
- Update the tech-debt register and `docs/track-ed-integration.md` when the
  legacy layout model is removed.
