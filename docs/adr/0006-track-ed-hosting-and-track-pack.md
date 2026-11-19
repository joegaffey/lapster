# 0006. Host track-ed as a submodule and share the track-pack compiler

- Status: accepted
- Date: 2026-11-19
- Deciders: Architect, @joegaffey
- Related: PR #34, `docs/track-ed-integration.md`, ADR 0003, ADR 0004

## Context

track-ed is the browser-first venue editor. Lapster already had two separate
paths for GLB track data: the native `asset-compiler` importer and the browser
editor's procedural baker. Letting track-ed drive its current track required a
byte-based GLB importer that could run both natively and in the browser without
a server-side compile step.

Embedding the editor also changes the build topology: the Pages build must
produce the editor host, the drive page must load a locally stored pack, and the
legacy procedural editor remains in service during migration.

## Decision

- Keep track-ed as a pinned git submodule at `web/track-ed-src`, built at Pages
  time with `VITE_LAPSTER_HOST=1` and copied to `dist/web/track-ed/`.
- Extract the GLB track importer and `LSIM` pack builder into the shared
  `crates/track-pack` crate. `crates/track-pack-wasm` exposes it to the browser
  as `compile_glb_track(track_json, glb, id) -> Uint8Array`.
- The browser stores the compiled pack under the local IndexedDB key
  `lapster.track-ed.v1` with pack ID `track-ed`. Drive resolves
  `?track=track-ed` from that key; `?track=editor` remains the legacy
  procedural path.
- Make track-ed the primary **Track Editor** entry and keep the procedural
  editor as **Legacy Track Editor** until removal is decided separately.
- Pin the submodule to track-ed `main` and ship the track-ed license and
  third-party notices with the built site.

## Consequences

### Positive

- track-ed Test drive works without an offline export, CLI, or server compile.
- Native and browser pack output share one implementation and one validation
  path.
- The editor host is built from a reproducible pinned submodule at deploy time.

### Negative / risks

- Production builds depend on the track-ed remote and its pinned `main` commit.
- Embedded track-ed and its bundled dependencies increase the set of licenses
  that must be shipped with the Pages site.
- The legacy editor remains as duplicated surface area until a removal decision
  is made.
- Hosted track-ed and the file-import page have limited automated browser
  coverage; PR CI still runs only the Lapster smoke subset.

## Alternatives considered

- Compile track-ed exports with the native `asset-compiler` CLI: rejected
  because it breaks the one-click browser flow.
- Write the pack format in TypeScript: rejected as a second, divergent pack
  writer.
- Vendor the track-ed sources into this repository: rejected in favor of a
  pinned submodule with its own build and tests.
- Sandbox track-ed in an iframe without host integration: rejected because
  Test drive needs the compiled pack and versioned Lapster wasm URL.

## Follow-up

- Remove the legacy procedural editor only after track-ed has production use.
- Add smoke coverage for the hosted track-ed Test drive page.
- Revisit the submodule pin when track-ed begins making independent releases.
