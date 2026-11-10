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
| D1 | render/web | The independent-loop track prototype was superseded by the loop/shared-control-point model. | Replaced by `@3` loops, shared control points, continuous full-loop sampling, and Ctrl+click branch drafts. | P1 | multi-circuit batch | architect | resolved | Local WIP checkpoint committed; junction/pit polish remains. | 2026-11-23 |
| D2 | web | Audio Playwright tests are unstable locally: worklet level/gain thresholds fail and full viewer runs can hang under WSL/software rendering. | Gate noise; full viewer suite is not reliably green outside CI. | P2 | multi-circuit batch | web | open | Reproduce on a clean CI-like environment or split audio tests from the default viewer gate. | 2026-11-23 |
| D3 | web | Playwright project-suffixed screenshot baselines duplicate the legacy unsuffixed files. | Redundant test assets. | P3 | multi-circuit batch | web | open | Remove the unsuffixed copies after confirming CI uses the suffixed names. | 2026-12-07 |
| D4 | assets | `fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima` was schema-patched instead of re-baked because assimp is unavailable. | Pack content and schema are valid, but the source pipeline was not exercised. | P2 | schema v4 migration | assets | open | Install assimp and run `tools/speed-dreams/build-car.sh`; re-bake the pack. | 2026-11-23 |
| D5 | assets | Re-baked `skidpad05` visual bytes differ from the previous committed pack because the local `fixtures/real` source changed. | Large noisy fixture diff; bake is deterministic against current source. | P3 | schema v4 migration | assets | accepted | Keep the current pack; document the raw-source version if needed. | 2026-12-07 |
| D6 | web | Editor project migration from `@1`/`@2` to `@3`. | Saved local projects migrate through `EditorProject::from_json`. | P2 | loop model | web | resolved | Keep migration coverage when the editor tests are updated. | 2026-11-23 |
| D7 | web | Track-editor Playwright tests still reference removed `#branch-*` and `#duplicate-layout` controls. | Editor test suite is stale and cannot be used as a gate. | P1 | loop model | web | open | Update tests for Ctrl+click branch, all-loop overlay, and loop controls. | 2026-11-23 |
| D8 | render | Branch junction pads and prop/barrier placement are rough around merged edges. | Visual seams/overlaps near junctions. | P2 | loop model | render | open | Refine junction pads and suppress crossing props near junctions. | 2026-11-23 |

## Lifecycle

- `open` -> `scheduled` -> `in progress` -> `resolved`
- `open` -> `accepted` (with review date and ADR if significant)
- `open` -> `superseded` (link the new ID)

Resolved items link to the PR or commit. Superseded items link to the new ID.
