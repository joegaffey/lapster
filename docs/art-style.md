# Lapster Art Style: Technical Blueprint / Papercraft Toon

Status: adopted. CP4.5 implements the flat-screen forward-renderer slice; WebXR is later.

## 1. Goals

Lapster should feel like a live engineering illustration: simple, readable vehicle
silhouettes, crisp hard-edged geometry, and restrained technical detailing. The reference
point for car geometry is Live for Speed-style simplicity, not high-fidelity CAD.

The style must:

- stay readable at speed,
- render in a single forward pass with no G-buffer,
- work without screen-space outlines or post-processing,
- keep texture memory and fill-rate costs low,
- remain suitable for VR later without changing the core shading model.

## 2. Rendering Approach

Forward rendering is the target.

- One directional key light plus ambient/fill light.
- Toon shading in the fragment shader:
  - quantized diffuse steps,
  - optional rim term,
  - optional cheap specular highlight,
  - no physically based BRDF requirement.
- Vertex colors are the default surface treatment.
- Optional base-color textures are sampled in the same forward pass. They are allowed where
  they are the practical choice.
- No G-buffer, SSAO, bloom, screen-space edge detection, or other post-processing.
- MSAA is preferred over post-process anti-aliasing.
- Shadows use a focused directional shadow map with soft PCF filtering. Keep the map
  resolution and distance tight enough for stable, artifact-free results.

## 3. Outlines and Technical Lines

Use object-space geometry, not screen-space filters.

- **Inverted hull:** a second mesh expanded along normals, front-face culled, rendered before
  the base mesh.
- The base mesh uses split vertices/normals at hard edges for flat facets.
- The outline hull uses a separate welded buffer with averaged normals so hard-edge faces do
  not separate into gaps.
- Offset width should be distance-scaled so lines stay readable near and far.
- Use depth bias to avoid z-fighting at silhouettes.
- Outline hero objects and nearby barriers. Do not outline every piece of distant track
  geometry; static edges should be baked into geometry or handled with limited edge strips.
- Use procedural world-space grid lines on flat track surfaces with derivative-based
  anti-aliasing and distance fade. Avoid grids on curved car bodies and steep surfaces where
  they stretch or shimmer.

## 4. Textures: Allowed, Budgeted, Purposeful

Textures are not banned. They are a targeted tool:

**Use textures for:**

- dashboard gauges, dial faces, labels, and warning lights,
- signs, banners, pit boards, and track advertising,
- decals, license plates, and small livery details,
- small sprites where geometry is a poor fit.

**Do not use textures for:**

- full PBR sets (normal, roughness, metalness, occlusion),
- high-resolution tiling surfaces that vertex colors can express,
- large alpha-blended billboards,
- anything that forces a texture-heavy material permutation explosion.

**Texture budget and rules:**

- Prefer one or two atlases (for example 1024² or 2048²) for cockpit, signs, and decals.
- Use KTX2/Basis or PNG with mipmaps.
- Decode color textures as sRGB; keep vertex and material colors linear.
- Apply the existing linear-to-sRGB output conversion exactly once.
- Alpha-test small decals/signs. Alpha blending is limited to small screen coverage; no large
  transparent surfaces in the forward pass.
- Meshes without textures may omit UVs entirely.
- A textured material may use vertex color as a tint.

## 5. Geometry and Color

- Car base mesh: target ≤5,000 triangles, ≤16 detachable parts.
- Hull mesh: budget similar to the base mesh; only generate it for objects that need
  outlines.
- Use flat/split normals for hard faceted surfaces; do not rely on smooth normal
  interpolation for the papercraft look.
- Vertex colors are `rgba8unorm` (linear) where practical. Material base colors remain a
  fallback for uncolored meshes.
- Keep the palette small and deliberate. A small set of flat colors, plus black/charcoal for
  outlines, is enough.
- Track architecture should use clean planes, strong bevels, and instancing. Set a
  visible-chunk triangle and draw-call budget and validate it in the asset compiler.
- Preserve component IDs and anchor transforms for detachable parts.

## 6. Real-Asset Conversion

AC car and track assets are not authored for this style. The offline pipeline must:

1. Decimate meshes to the style budget while preserving silhouette.
2. Split vertices for flat normals.
3. Bake material/texture color into a small palette or per-face/per-vertex colors.
4. Generate the welded outline-hull buffer.
5. Tag detachable parts, their anchors, and their damage thresholds.
6. Convert textures only for gauges, signs, decals, and similar exceptions.
7. Validate triangle counts, part counts, UV/texture usage, and normal splits.

This is a technical-art pipeline, not a runtime shader trick.

## 7. Destruction and Deformation

Start with a data-driven part system, not runtime mesh cutting.

- Add a fixed-size part status bitmask and per-part transform state.
- Detached parts become visual shard instances with a small fixed pool.
- Shards are visual-only initially; authoritative shard rigid bodies can come later.
- Do not put per-vertex offset arrays for a whole car in the 4 KB SAB snapshot. If
  deformation is needed, use a small set of shader-side dent parameters or a separate
  preallocated shared buffer.
- No heap allocation in the physics tick.

## 8. Cockpit and Telemetry

- Dashboard, dial faces, needles, and switch gear are 3D geometry.
- Gauge faces and labels may use small textures.
- Prefer 7-segment geometry or vector glyph meshes for digits; avoid reintroducing a large
  font atlas.
- Drive needles with transforms from telemetry (`engine_rpm`, `steering_rack_torque`,
  `slip_ratio`, wheel loads, gear).
- No canvas overlay in XR later; flat-screen HUD may remain a DOM layer.

## 9. Particles

- Tire smoke: opaque or alpha-tested low-poly puffs, instanced. Grass surfaces emit a
  tapered blade-shaped billboard instead of the puff.
- Sparks: thin bright geometry, instanced, additive blending.
- Use fixed-size instance buffers and hard caps.
- Do not use large alpha-blended sprites. Instancing removes draw-call cost, not overdraw.
- Fade by scale/color or dithering rather than large transparent coverage.

## 10. WebXR (Later)

WebXR is deferred. When it returns:

- Validate WebXR support on the target runtime with the chosen renderer before promising
  stereo support.
- Forward rendering is already the right base; avoid adding deferred shading.
- Start with two-pass per-eye rendering. Use multiview only if the runtime, adapter, and wgpu
  path support it.
- Move physics to the worker/SAB topology before VR latency work.
- Treat 90 Hz stereo as a measured target, not an assumption.

## 11. Initial Budgets

These are targets to validate, not guarantees:

| Area | Target |
| --- | --- |
| Car base mesh | ≤5k triangles |
| Car outline hull | ≤5k triangles |
| Detachable car parts | ≤16 |
| Cockpit texture atlas | 1024²–2048² |
| Sign/decal atlas | 1024²–2048² |
| Texture formats | KTX2/Basis or PNG with mipmaps |
| Opaque particle instances | ≤2,000 |
| Spark instances | ≤2,000 |
| Large alpha-blended sprites | zero |
| Post-processing passes | zero |
| Flat-screen target | 60 fps |
| WebXR target | 90 Hz later, measured |

Track visible-chunk triangle and draw-call budgets should be set after the first converted
track is measured.

## 12. Phases

1. **Style slice — done:** toon ramp/rim shading, flat vertex colors, inverted-hull outlines,
   and a UV/base-color texture path with a generated gauge sign. An instanced,
   wind-animated toon grass pass now covers the Skidpad; the procedural grid and particles
   remain future work.
2. **Real car:** decimate/color-bake a simple LFS-like car, add cockpit gauges and signs.
3. **Destruction:** part bitmask, shard pool, damage transitions.
4. **WebXR:** runtime spike, per-eye budgets, worker physics.

## 13. Validation

- Asset compiler checks triangle/part/color budgets and hull generation.
- Golden screenshots for the flat WebGPU path, plus WebGL2 fallback where supported.
- Per-eye XR screenshots and frame-time traces only when XR work begins.
- No screen-space outline or post-processing dependency.
