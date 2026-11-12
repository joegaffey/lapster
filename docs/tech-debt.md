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
| TD-0001 | web | Track-editor kerb/prop decoration painting was removed with the old per-segment span setters; the `@5` project model stores canonical decoration paths and there is no UI to author them. | Decoration can no longer be authored in the editor, though baked decoration and existing projects still render and drive. | P1 | Phase 3b-1 | web | open | Build decoration-path painting on `set_editor_project` in Phase 3b-3. | 2026-12-12 |
| TD-0002 | render | `settings.width` is validated but the bake uses the constant `ASPHALT_HALF_WIDTH`; changing the global width has no effect. | Authors cannot change road width yet, and per-point width cannot be added safely on top. | P2 | `@5` audit | render | open | Wire global width through sampling, kerbs, props, racing line, collision, and decoration lanes, or remove the setting until it is supported. | 2027-01-12 |
| TD-0003 | render/web | Per-control-point width and asymmetric profiles are deferred by ADR 0003; decoration lanes assume a constant width. | Venue authoring cannot vary track width at a point yet. | P2 | ADR 0003 | architect | scheduled | Design a width/profile contract and ADR before implementing variable-width bake and editor handles. | 2027-02-12 |

## Lifecycle

- `open` -> `scheduled` -> `in progress` -> `resolved`
- `open` -> `accepted` (with review date and ADR if significant)
- `open` -> `superseded` (link the new ID)

Resolved items link to the PR or commit. Superseded items link to the new ID.
