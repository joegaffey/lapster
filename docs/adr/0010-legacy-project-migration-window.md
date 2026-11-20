# 0010. Migrate legacy projects with a one-release read-only window

- Status: accepted
- Date: 2026-11-20
- Deciders: architect, @joegaffey (product owner)
- Related: ADR-0007, `docs/features/editor-consolidation-plan.md`,
  `docs/editor-consolidation-report.md`, TD-0014, `web/track-editor/`

## Context

ADR-0007 makes track-ed the sole supported editor and gates removal of the
legacy procedural editor on a legacy-project migration decision. The two
formats are not interoperable:

- the legacy editor uses `lapster/track-project@6`, with global points,
  layouts, start/finish, sectors, route legs, and its own decoration arrays;
- track-ed uses `track-ed/project@5`, with sections, junction members,
  first-class roadside splines and runs.

Legacy projects currently live only in the browser:

- `web/track-editor/main.ts` stores one project under
  `lapster.track-editor.project.v6` in `localStorage`;
- the legacy page already supports **Export** to normalised
  `lapster/track-project@6` JSON and **Import** from a local file;
- there is no server-side project store and no legacy-project fixture corpus;
- track-ed does not import `lapster/track-project@6`.

A converter is possible, but it is medium effort: it must map global points,
sections, junctions, layouts, start/finish, sectors and decorations, collapse
multiple layouts, and report lossy conversions. There is no evidence of a
meaningful shared project corpus that justifies that work now.

## Decision

Choose **Option B: a one-release read-only legacy window.**

- Remove the legacy editor from the primary navigation now. The **Track
  Editor** card remains the supported entry point.
- Keep `/web/track-editor/` reachable for one deprecation window so users can
  export their `lapster/track-project@6` JSON or re-author in track-ed.
- Add a visible deprecation notice to the legacy page with an export/re-author
  instruction and a link to track-ed.
- Do not build a `lapster/track-project@6` → `track-ed/project@5` converter in
  this window.
- Remove the legacy page and its editor-only Rust/TypeScript implementation
  after the window, subject to the final ADR-0007 parity check and a follow-up
  ADR recording the removal.

The window ends at the later of:

- the first production release after this ADR is merged, or
- 30 days after that release.

The architect reviews removal no earlier than **2026-12-20**. Earlier removal
requires an explicit product-owner decision.

## Consequences

### Positive

- Legacy projects remain recoverable through the existing JSON export for one
  release, without building and maintaining a schema converter.
- The single-editor direction and navigation stay clear: track-ed is the
  supported authoring path.
- The legacy editor's writer, validators, renderer APIs and tests can be
  deleted once, after the window, with a documented data story.

### Negative / risks

- The legacy editor and its editor-only wasm APIs remain in the build for one
  release.
- Users who miss the window cannot import `lapster/track-project@6` into
  track-ed; it is a documented break.
- A future demand for legacy projects would require a separate conversion ADR.

## Alternatives considered

- **Option A — one-time converter now.** Best long-term user outcome, but
  medium effort, requires its own ADR/tests, and is not justified without
  evidence of a project corpus that must be preserved.
- **Option C — declare legacy projects unsupported and delete now.** Lowest
  effort, but it removes the only browser-local copy path before users can
  export. Rejected for this window.

## Follow-up

1. Remove the legacy card from `web/index.html` (done with this ADR).
2. Add the deprecation/export notice to `web/track-editor/`.
3. Keep track-ed autosave and the hosted/CI gates in place as the replacement
   data path.
4. After the window, delete `web/track-editor/`, legacy editor-only Playwright
   cases, editor-only APIs in `crates/sim-render/src/lib.rs`, and
   `crates/sim-render/src/track_editor.rs`.
5. Record the final removal and the migration outcome in a follow-up ADR, and
   update `AGENTS.md`, `PROGRESS.md`, the domain map and `docs/tech-debt.md`.
