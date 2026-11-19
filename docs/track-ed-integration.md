# track-ed integration

Status: integrated on `dev` in PR #34 (`52a2dc6`). The submodule is pinned to
track-ed `main`, and the Pages build ships the track-ed notices and licenses.
Release to `main` is the remaining step. See ADR 0006.

## What is included

- `crates/track-pack`: byte-based GLB track importer and `LSIM` pack builder. It is the
  shared compiler used by the native asset compiler and the browser.
- `crates/track-pack-wasm`: lazy wasm wrapper exposing:
  `compile_glb_track(track_json, glb, id) -> Uint8Array`.
- `web/track-ed-src`: pinned track-ed submodule. The Pages build compiles it with
  `VITE_LAPSTER_HOST=1` and copies it to `dist/web/track-ed/`, together with the
  track-ed license and third-party notices.
- `web/track-ed/`: hosted track-ed. Its **Test drive in Lapster** action compiles the
  current export, stores it under `lapster.track-ed.v1`, and opens
  `/web/drive/?track=track-ed`.
- `web/track-import/`: file-based import for `track.json` + `track.glb`. It uses the same
  compiler and local pack key, so it does not require embedding track-ed.
- Drive resolves `?track=track-ed` from the local pack key. The existing
  `?track=editor` path is unchanged.
- The menu makes track-ed the primary **Track Editor** and keeps the procedural editor as
  **Legacy Track Editor**.

## Build

The Pages workflow checks out submodules recursively, then:

```bash
cd web
npm ci
npm run build:viewer
cd track-ed-src
npm ci
VITE_LAPSTER_HOST=1 npm run build
cd ..
bash scripts/build-pages.sh
```

`build:viewer` builds `track-pack-wasm` into `web/pkg/track-pack/`. `build-pages.sh`
requires `web/track-ed-src/dist/` and copies it to `dist/web/track-ed/`.

For local development:

```bash
cd web/track-ed-src
npm ci
VITE_LAPSTER_HOST=1 npm run build
cd ..
bash scripts/build-pages.sh
python3 -m http.server 8123 --bind 127.0.0.1
```

Then open `http://127.0.0.1:8123/dist/web/track-ed/`.

## Local pack contract

- Track ID: `track-ed`
- IndexedDB database/store: `sim-assets` / `packs`
- Cache key: `lapster.track-ed.v1`
- Pack manifest ID: `track-ed`

The compiler hashes the supplied `track.json` and `track.glb` bytes, ignores the manifest's
`visual.glb` path, and does not read the filesystem. This makes native and browser output
deterministic for the same inputs.

## Tests

- `cargo test -p asset-compiler --test round_trip`
- `cargo test -p track-pack --test byte_compile`
- `cd web && npx playwright test tests/track-import.spec.ts --project=default`
- Manual: hosted track-ed **Test drive** and `web/track-import/` both open drive with
  `asset.id = "track-ed"`.

## Remaining migration work

The legacy editor is retained. Removing it is a separate product decision after track-ed has
been exercised in production. `.gitmodules` now tracks track-ed `main`, and the pin is the
merged `main` commit `382b311`. ADR 0006 records the hosting and `track-pack` decision.
