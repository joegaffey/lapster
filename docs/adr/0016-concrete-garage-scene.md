# 0016. Dedicated concrete garage scene with overhead spotlights

- Status: accepted
- Date: 2026-11-22
- Deciders: architect, @joegaffey
- Related: ADR-0002, `agents/contracts.md`, `docs/shadows.md`, `README.md`

## Context

The Garage page builds the selected track scene and places the car catalogue
across ten bays in two facing rows of five. The requested garage is a
purpose-built venue instead: a concrete floor, a spotlight over each car, and
the cars in a single line. The renderer currently has one directional sun, a
single environment shadow map, and car silhouette decals; it has no local light
model. The web bay navigation also assumes the two-row snake order.

The Garage page is the only garage surface today. The change therefore has to
choose between replacing the track-backed showroom and adding a second
presentation. The request says "add", so the showroom is retained.

## Decision

1. Add an additive garage presentation mode, `concrete`, and keep the current
   track-backed showroom as `showroom`.
   - The Garage page defaults to `concrete`.
   - `?garage=showroom` selects the existing showroom; `?track=` continues to
     choose its backdrop.
   - No new page, route, menu entry, or registry contract is added.

2. Concrete mode does not build a track scene.
   - It builds a procedural concrete floor slab, a dark studio sky and ambient
     level, one emissive lamp housing and one downward spot light per bay, and
     a single straight row of cars facing the viewer.
   - Cars share one orientation and a constant spacing.
   - The floor is placed at the cars' wheel-contact height.

3. Add a garage lighting path to the renderer.
   - Up to ten spot lights are carried in the camera uniform and evaluated in
     the mesh fragment shader.
   - The directional sun remains the shadow caster, fixed vertically for
     concrete mode, so the existing environment shadow map and car silhouette
     decals continue to provide contact shadows.
   - Spot lights do not cast shadows in this iteration.

4. Keep the wasm viewer interface explicit.
   - Add `GarageViewer.create_concrete(canvas, car_packs)` for concrete mode.
   - Keep `GarageViewer.create(canvas, track_pack, car_packs)` for showroom.
   - `bay_info()` exposes `layout: "line" | "grid"`.
   - `debug_info()` reports the garage mode, floor geometry, and spot-light
     count.

5. Persist the mode only in the URL.
   - `?garage=showroom` is the opt-out; the default is concrete.
   - The selected car and track persistence is unchanged.

## Consequences

### Positive

- The Garage page has a purpose-built concrete venue with overhead lighting.
- The showroom and its track backdrop remain available.
- No asset-schema, ABI, or protocol contract changes.
- The spotlight path is small and reuses the existing shadow and decal paths.

### Negative / risks

- Two garage scene paths and two sets of browser expectations to maintain.
- Spot lights do not cast shadows; contact shadows come from the overhead sun.
- The camera uniform grows by the spot-light array.
- Screenshot baselines and several garage specs must be updated.

## Alternatives considered

- **Replace the showroom entirely.** Simpler, but removes an existing feature
  and conflicts with the "add" request.
- **One spotlight on the selected car only.** Reads as a single moving lamp,
  but does not light the rest of a single-line lineup.
- **Fake spotlights with emissive floor pools and no shader lights.** Cheaper,
  but the car and floor would not actually be lit from above.
- **Spot-light shadow maps.** More physically complete, but unnecessary for a
  stylized studio and too costly for this iteration.

## Follow-up

- Update `README.md`, `docs/shadows.md`, and `PROGRESS.md`.
- Add browser coverage for concrete default, single-line order, and the
  showroom fallback.
- Revisit spot-light shadows only if the studio look requires them.
