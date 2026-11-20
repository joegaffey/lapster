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
| TD-0006 | web/render | Seam-wrapped decoration authoring is implemented, but the editor still lacks custom-lane authoring (deliberately dropped from the UI for now) and junction chain continuation across multi-leg routes. | Multi-section venues still need adjacent legs painted separately, long junction chains cannot be authored as one unit, and custom prop lanes cannot be authored yet. | P2 | `@6` painter | web | scheduled | Seam paint and seam endpoint resize are complete and covered by Playwright (`painting across a closed-section seam wraps the kerb leg`, `resizing a kerb endpoint across a closed-section seam wraps the leg`). Remaining: custom-lane authoring (dropped for now) and junction chain continuation, blocked on multi-leg routes (Phase 4); re-evaluate after Phase 4. | 2027-01-12 |
| TD-0007 | web | Synchronous wasm editor getters (`editor_project()`, `editor_points()`, etc.) panic with "recursive use of an object" if called while `set_editor_project()` is in flight; the production render loop is guarded by `suspended`, but tests and future callers must wait for the rebuild. | A careless read during an async rebuild crashes the editor/test harness. | P3 | Step 0b web | web | accepted | Keep reads behind the `rebuilding`/`suspended` guard; prefer a settled-state helper in tests. Review when the editor API is refactored. | 2027-05-14 |
| TD-0008 | render | Junction branches and host sections are baked as independent full-width ribbons; at angles the overlapping dirt/grass/tarmac produces z-fighting, holes, and discontinuous collision. Local trim/lift/cut patches do not generalize. | The legacy editor is broken at angled joins; track-ed is the supported path. | P1 | `@6` junctions | render | superseded | Track-ed's planar-arrangement junction bake is the supported solution (ADR-0007); legacy removal is tracked in `docs/features/editor-consolidation-plan.md`. Do not continue legacy junction geometry. | 2026-12-20 |
| TD-0009 | render/web | Imported track-ed conifer trunks render gray/lavender even though the GLB `tree_trunk` material contains a dark-brown solid texture (sampled pixel ~`#5D3018`) and the preview uses the same source color. The gray bars may be either incorrectly shaded trunks or the nearby concrete block run (blocks sit at offset 9 m, trees at 12/16 m), so the first step is to isolate a tree-only pack. | Tree trunks do not read as brown in the Lapster track viewer, weakening the conifer look and making trunks easy to confuse with gray prop blocks. | P2 | track-ed conifer refresh | render | open | Load a tree-only `track-ed` GLB in the track viewer and inspect mesh/material assignment, UVs, and rendered pixel colors; confirm whether the gray pixels are trunks or blocks, then fix the material import or the toon/rim shading path and add a visual regression check. | 2027-01-12 |
| TD-0010 | track-ed app | `attachEditing` is ~1,500 lines and `createTrackView` is ~1,360 lines; pointer transitions and renderer/overlay/picking logic are concentrated in two functions. | UI changes are risky, review is hard, and browser interaction coverage is weak. | P2 | editor consolidation report | web | scheduled | Split by mode/handler and into renderer/overlays/picking/lifecycle; add browser smoke coverage. See `docs/features/editor-consolidation-plan.md`. | 2026-12-20 |
| TD-0011 | track-ed app | Hidden Kerbs/Walls/Props modes, panels, drag states and update functions remain wired alongside Roadside mode. | Roughly 1,000 lines of redundant UI and a second authoring path create confusion and regression risk. | P2 | editor consolidation report | web | scheduled | Remove once `project@5` is the only authoring path; verify Roadside painting after removal. | 2026-12-20 |
| TD-0012 | track-ed CI | No hosted track-ed browser test, no track-ed unit/build run on submodule pin changes, and no dependency/licence audit gate. | Track-ed or hosted Test drive regressions can land silently in Lapster. | P1 | editor consolidation report | architect/web | scheduled | Add hosted Playwright smoke, track-ed CI on pin changes, and `npm audit`/notice checks. | 2026-12-20 |
| TD-0013 | track-ed kernel | Large-venue bake cost is unbenchmarked; documented synthetic cases reach 2.0 s, 9.7 s and 31.1 s for 4, 9 and 16 sections. | Large tracks may stall the authoring loop as venues grow. | P3 | editor consolidation report | track-ed | scheduled | Benchmark an 8–16-section venue; optimise, then move bake to a Worker if needed; port the kernel only with profiling evidence. | 2027-01-20 |
| TD-0014 | migration | Legacy `lapster/track-project@6` projects have no migration path to track-ed. | Legacy editor removal is blocked until the migration policy is decided. | P2 | editor consolidation report | architect | open | Choose converter, read-only deprecation window, or documented unsupported archive (ADR-0007 gate). | 2026-12-20 |

## Lifecycle

- `open` -> `scheduled` -> `in progress` -> `resolved`
- `open` -> `accepted` (with review date and ADR if significant)
- `open` -> `superseded` (link the new ID)

Resolved items link to the PR or commit. Superseded items link to the new ID.
