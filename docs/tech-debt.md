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
|  |  |  |  |  |  |  |  |  |  |

## Lifecycle

- `open` -> `scheduled` -> `in progress` -> `resolved`
- `open` -> `accepted` (with review date and ADR if significant)
- `open` -> `superseded` (link the new ID)

Resolved items link to the PR or commit. Superseded items link to the new ID.
