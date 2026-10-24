# Shadow modes and time of day

## Goals

- Keep the current dynamic shadow quality as **High**.
- Add a cheap **Baked** environment mode.
- Make car shadows independently selectable: dynamic, silhouette decal, or off.
- Bake at startup for the selected time of day so high-resolution static shadows
  do not cost per-frame runtime.
- Keep the graphics settings available in Drive, Garage, and Track Viewer.

## Settings

Shadow settings are global and persisted in `lapster.settings.v2`. The dev Pages build
uses `lapster.settings.dev.v2` and defaults to Baked environment shadows plus the
Silhouette car shadow; stable builds keep Auto.

### Environment shadows

- **Auto** - High on desktop WebGPU, Baked on mobile and WebGL2.
- **High** - current dynamic focused light-space map with 4-tap PCF.
- **Baked** - a high-resolution static light-space shadow map baked once at startup;
  sampled with 4-tap PCF and no per-frame shadow pass.
- **Off** - no environment shadows.

### Car shadow

- **Auto** - Dynamic with environment High, Silhouette otherwise.
- **Dynamic** - car cast into the dynamic shadow map where available.
- **Silhouette** - baked car silhouette decal; no per-frame car shadow map.
- **Off** - no car shadow.

Garage cars are static, so Dynamic falls back to Silhouette there. Track Viewer has no
car, so the car setting is ignored.

## Settings dialog

The Settings dialog is available in all three viewers.

- **Gameplay** tab (Drive only): TC, TC strength, ABS, ESC, ESC strength, steering
  sensitivity.
- **Graphics** tab: anti-aliasing, environment shadows, car shadow, time of day,
  debug overlay, and the FPS counter.

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
  - Drive/Track Viewer: around the track collision bounds plus margin.
  - Garage: the showroom.
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
  - offset along the light's horizontal direction by
    `body_height * horizontal_light / vertical_light`;
  - optional stretch at low sun angles;
  - offset 1-2 cm above the ground.
- The offset is recomputed when the car moves or time of day is applied, so the decal
  moves around the car with the sun direction.

## Dynamic car shadow

- High + Dynamic: the car is included in the High environment map; no extra pass.
- Baked/Off + Dynamic: the same map is focused tightly around the car (about 15 m)
  and contains car casters only.
- 4-tap PCF is used only for the car map; High environment keeps its current 4-tap PCF.
- The dynamic map is refreshed only when the car transform, focus, or sun changes.

## Renderer resources

- Dynamic map: existing depth map with comparison sampler.
- Baked map: static high-resolution depth texture using the existing comparison sampler.
- Car decals: shared silhouette texture plus a decal pipeline.
- Static and dynamic caster flags distinguish track from car meshes.
- Temporary bake targets are released after baking; the active baked map is retained
  for mode switches.

## Telemetry

Expose requested/applied environment and car modes, time of day, sun elevation,
baked map resolution, bake time, and shadow memory estimate.

## Acceptance

- High mode visual quality does not regress.
- Baked + Silhouette performs no per-frame environment shadow pass and uses 4-tap
  PCF against the static map plus one decal draw per car.
- Baked + Dynamic samples the car-only map only near the car.
- Car silhouette follows car heading and sun direction and disappears when car shadow
  is Off.
- Time-of-day Apply updates sky tint, sun direction, and shadows; Baked mode rebakes.
- Settings tabs work with keyboard and mouse, persist, and apply in all viewers.
- WebGL2 smoke test passes; 4096 falls back to 2048 where unsupported.
