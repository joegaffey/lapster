# Shadow modes and time of day

## Goals

- Keep the current dynamic shadow quality as **High**.
- Add a cheap **Baked** environment mode.
- Make car shadows independently selectable: dynamic, silhouette decal, or off.
- Bake at startup for the selected time of day so high-resolution static shadows
  do not cost per-frame runtime.
- Keep the graphics settings available in Drive, Garage, and Track Viewer.

## Settings

Shadow settings are global and persisted in `lapster.settings.v3`; the dev Pages build
uses `lapster.settings.dev.v3`. Both desktop and mobile default to Baked environment
shadows plus the Dynamic car shadow. There is no Auto mode; legacy stored Auto values are
coerced to Baked and Dynamic, while explicit modes are preserved.

### Environment shadows

- **High** - current dynamic focused light-space map with 4-tap PCF.
- **Baked** - a high-resolution static light-space shadow map baked once at startup;
  sampled with 4-tap PCF and no per-frame shadow pass.
- **Off** - no environment shadows.

### Car shadow

- **Dynamic** - car cast into the dynamic shadow map where available.
- **Silhouette** - baked car silhouette decal; no per-frame car shadow map.
- **Off** - no car shadow.

Garage cars are static, so Dynamic and Silhouette both use the environment shadow map,
which includes the car meshes; Off removes them from that map. Track Viewer has no car,
so the car setting is ignored.

## Settings dialog

The Settings dialog is available in all three viewers.

- **Gameplay** tab (Drive only): TC, TC strength, ABS, ESC, ESC strength, steering
  sensitivity.
- **Graphics** tab: render scale, anti-aliasing, environment shadows, car shadow, time
  of day, debug overlay, and the FPS counter.

Time of day uses a slider plus an Apply button. It only takes effect when applied, so
the Baked map is not rebuilt continuously.

## Time of day

- Slider range: 06:00-20:00, 30-minute steps.
- Sun elevation is clamped above the horizon so shadows remain valid.
- Apply updates the sun direction, sky tint, dynamic shadow requests, car silhouette
  offset, and rebakes the Baked environment map when necessary.
- Baked rebake time is reported in the settings status and telemetry.

## Baked environment

- Startup build: static casters only; the moving car is excluded.
- Region:
  - Drive: around the settled vehicle spawn.
  - Track Viewer: around the track spawn.
  - Garage: the showroom.
  - The region spans the track extent capped at 220 m per side, so very large tracks
    keep useful local resolution instead of baking the entire level.
- Resolution: 4096 on desktop WebGPU when supported, otherwise 2048; 2048 on mobile
  and WebGL2.
- Format: static Depth32Float light-space shadow map.
- Bake: render static casters once to the high-resolution directional depth map.
- Runtime: 4-tap PCF against the static map; no per-frame shadow depth pass.
- Higher bake resolution improves edge fidelity. The runtime sample count is unchanged;
  the tradeoff is VRAM: 4096² D32 is 64 MB, 2048² is 16 MB.
- The map is rebuilt only when lighting is applied or the viewer region changes.

## Car silhouette

- Generated once at load from the car body mesh; no external asset.
- Texture: 256x512 RGBA with a soft alpha edge, shared by all cars of the same model.
- Rendered with a dedicated unlit transparent decal pipeline.
- Transform:
  - position at the average contact plane, or wheel-bottom plane in Garage;
  - rotate with car yaw;
  - anchor the up-light edge at the car footprint;
  - stretch away from the sun along the light's horizontal direction as the sun lowers,
    using `body_height * horizontal_light / vertical_light`;
  - offset 1-2 cm above the ground.
- The stretch is recomputed when the car moves or time of day is applied. The shadow stays
  attached to the car instead of translating away from it.

## Dynamic car shadow

- High + Dynamic: the car is included in the High environment map; no extra pass.
- Baked/Off + Dynamic: a dedicated car-only map is centered on the car and contains
  car casters only.
- Car map resolution: 4096 on desktop, 2048 on mobile.
- The map focuses between the car and the end of its shadow, with an 8-14 m extent
  derived from sun elevation so long evening shadows stay inside the map.
- Car shadows use 32-tap PCF for a softer edge; the High environment keeps 4-tap PCF.
- The car map is refreshed only when the car transform, focus, or sun changes.

## Renderer resources

- Environment map: existing depth map with comparison sampler.
- Car map: dedicated higher-resolution depth texture with its own bind group.
- Baked map: static high-resolution depth texture using the existing comparison sampler.
- Car decals: shared silhouette texture plus a decal pipeline.
- Static and dynamic caster flags distinguish track from car meshes.
- Temporary bake targets are released after baking; the active baked map is retained
  for mode switches.

## Telemetry

Expose requested/applied environment and car modes, time of day, sun elevation,
environment and car map resolutions, baked map resolution, bake time, and shadow
memory estimate.

## Acceptance

- High mode visual quality does not regress.
- Baked + Silhouette performs no per-frame environment shadow pass and uses 4-tap
  PCF against the static map plus one decal draw per car.
- Baked + Dynamic samples the car-only map only near the car.
- Car silhouette follows car heading, stays attached to the footprint, stretches at low
  sun angles, and disappears when car shadow is Off.
- Time-of-day Apply updates sky tint, sun direction, and shadows; Baked mode rebakes.
- Settings tabs work with keyboard and mouse, persist, and apply in all viewers.
- WebGL2 smoke test passes; 4096 falls back to 2048 where unsupported.
