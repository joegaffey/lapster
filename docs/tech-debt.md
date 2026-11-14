# Technical debt register

The architect owns this register. Workers and the reviewer may identify debt,
but only the architect records it here.

## Triage

The architect triages new debt at batch boundaries: fix now, schedule, accept,
or escalate. There is no fixed debt capacity per batch.

- `P0` blocks work; fix immediately.
- `P1` high; schedule soon.
- `P2` medium; schedule when it affects the area.
- `P3` low; accept or batch with related work.

`accepted` means won't fix. It needs a review date. Significant accepted debt
needs an ADR in `docs/adr/`.

## Register

| ID | Area | Description | Impact | Severity | Origin | Owner | Status | Next action | Review |
|---|---|---|---|---|---|---|---|---|---|
| TD-0001 | web | Track-editor kerb/prop decoration painting was removed with the old per-segment span setters; the `@5` project model stores canonical decoration paths and there is no UI to author them. | Decoration can no longer be authored in the editor, though baked decoration and existing projects still render and drive. | P1 | Phase 3b-1 | web | resolved | Decoration editor implemented in `@6`: `Points | Kerbs | Props`, renderer-derived guide rails, baked-default import, click-select, endpoint-handle resize, single-item inspector, paint on empty rails, overlap errors, and persistence. Cross-section chain/seam/custom-lane follow-up is TD-0006. | 2026-11-13 |
| TD-0002 | render | `settings.width` is validated but the bake uses the constant `ASPHALT_HALF_WIDTH`; changing the global width has no effect. | Authors cannot change road width yet, and per-point width cannot be added safely on top. | P2 | `@5` audit | render | open | Wire global width through sampling, kerbs, props, racing line, collision, and decoration lanes, or remove the setting until it is supported. | 2027-01-12 |
| TD-0003 | render/web | Per-control-point width and asymmetric profiles are deferred by ADR 0003; decoration lanes assume a constant width. | Venue authoring cannot vary track width at a point yet. | P2 | ADR 0003 | architect | scheduled | Design a width/profile contract and ADR before implementing variable-width bake and editor handles. | 2027-02-12 |
| TD-0004 | render | B-spline spans are clamped independently at shared anchors; exact C1 continuity across an anchor is not enforced, so a small tangent kink is possible at junctions. | Junction geometry may show a subtle tangent discontinuity. | P2 | ADR 0004 | render | open | Match anchor tangents across adjacent spans or switch to a knot-based B-spline evaluation. | 2027-01-12 |
| TD-0005 | web/render | Section overlay centerlines were sampled at 25 points while route centerlines use 120, so the selected section looked polygonal. | Editor selection highlight did not visually follow the road smoothly. | P2 | `@6` migration | render | resolved | Section overlays now sample at `SEGMENTS` (120); verify in the `@6` checkpoint. | — |
| TD-0006 | web/render | The decoration editor is single-section: cross-section chain continuation, seam wrap, custom-lane authoring, and multi-leg endpoint editing are not exposed. | Multi-section venues need adjacent legs painted separately, and long chains cannot be authored as one unit. | P2 | `@6` painter | web | scheduled | Bake now supports cross-section and seam-wrapped chains and Step 0b junction contacts exist; expose chain continuation, seam-wrap authoring, custom-lane controls, and multi-leg handle editing in the editor. | 2027-01-12 |
| TD-0007 | web | Synchronous wasm editor getters (`editor_project()`, `editor_points()`, etc.) panic with "recursive use of an object" if called while `set_editor_project()` is in flight; the production render loop is guarded by `suspended`, but tests and future callers must wait for the rebuild. | A careless read during an async rebuild crashes the editor/test harness. | P3 | Step 0b web | web | accepted | Keep reads behind the `rebuilding`/`suspended` guard; prefer a settled-state helper in tests. Review when the editor API is refactored. | 2027-05-14 |

## Lifecycle

- `open` -> `scheduled` -> `in progress` -> `resolved`
- `open` -> `accepted` (with review date and ADR if significant)
- `open` -> `superseded` (link the new ID)

Resolved items link to the PR or commit. Superseded items link to the new ID.
