# 0017. Replace the garage showroom with the concrete studio

- Status: accepted
- Date: 2026-11-22
- Deciders: architect, @joegaffey
- Related: ADR-0016, `docs/shadows.md`, `README.md`

## Context

ADR-0016 added the concrete studio as the default Garage presentation and kept
the track-backed showroom available through `?garage=showroom`. The product
owner reviewed the studio, the full 67GP catalogue, the paint picker, and the
studio wheel shading, and chose to make the concrete studio the only Garage
presentation. Keeping the fallback would preserve a second garage mode, a second
navigation order, a second screenshot path, and a track-pack load that the
Garage page no longer needs.

The track-backed scene builder is still required by Drive: `DriveViewer` builds
the track plus the player/AI/remote cars through `Scene::build_garage`.

## Decision

1. The Garage page always uses the concrete studio.
   - Remove the `?garage=showroom` mode and the showroom
     `GarageViewer::create` constructor from the wasm garage viewer.
   - Remove the Garage page's track-pack load and two-row snake navigation.
   - The Garage page always builds a single line, one bay per catalog car.

2. Keep the shared scene builder.
   - `Scene::build_garage` remains for `DriveViewer`; it is not dead code.
   - `DriveViewer` keeps its track scene, grid/spawn logic, AI/remote slots, and
     paint propagation.

3. Keep the follow-up garage work.
   - Eight-car catalogue: Mazda RX-7 once, all six 67GP cars, and the Deckard
     Conejo.
   - The data-driven paint picker remains, with ten palette colours for
     recolorable cars (currently the Mazda) and a "Factory livery" panel for
     textured cars.
   - The studio wheel shading uses the Drive-mode flat ramp
     (`0.45 + 0.55 * NdotL`, no rim/outline) instead of the spot/toon path.
   - Garage time-of-day controls are hidden; studio lighting is fixed.

4. Update the tests and docs.
   - Remove the showroom fallback specs and the reverse second-row navigation
     test.
   - Update the garage specs for eight bays, the paint picker, factory livery,
     and the hidden time-of-day control.
   - Update `README.md`, `docs/shadows.md`, and `PROGRESS.md`.
   - ADR-0016 is superseded by this record.

## Consequences

### Positive

- One Garage experience and one set of garage tests.
- The Garage page no longer downloads a track pack.
- The concrete studio, paint picker, and studio wheel shading are the shipped
  garage.
- Drive keeps the track-backed scene builder it needs.

### Negative / risks

- The old track-backed showroom is no longer viewable from the Garage page.
- The `Scene::build_garage` showroom code remains because Drive uses it; it is
  not reachable from the Garage viewer.
- A user with `?garage=showroom` bookmarked will get the concrete studio.

## Alternatives considered

- **Keep the additive showroom fallback.** Rejected by the product owner; it
  preserves two garage modes and a track-pack load the Garage page does not
  need.
- **Remove `Scene::build_garage` entirely.** Rejected; DriveViewer depends on
  it for the track scene and car rigs.
- **Keep a hidden query parameter for the old showroom.** Rejected as dead,
  undocumented UI.

## Follow-up

- Run the full workspace/web gate, merge PR #55 into `dev`, and record the merge
  in `PROGRESS.md`.
- Do not merge into `main` or deploy a Pages build without a separate request.
