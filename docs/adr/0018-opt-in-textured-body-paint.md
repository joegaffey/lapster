# 0018. Opt-in textured-body paint for minimal-livery cars

- Status: accepted
- Date: 2026-11-23
- Deciders: architect, @joegaffey
- Related: ADR-0017, `docs/speed-dreams-pipeline.md`,
  `crates/sim-render/src/garage.rs`, `web/src/ui/car-selection.ts`

## Context

ADR-0017 kept the ten-colour garage picker for vertex-painted cars (currently
the Mazda) and treated every textured body as a factory livery. The Deckard
Conejo is a textured car with a minimal red body livery, and the product owner
asked for the same fixed palette on the Deckard while keeping its decals and
lights intact.

Vertex recolouring cannot paint a textured body: the Deckard body material
samples a base-colour texture and its vertices are white. Repainting it
therefore needs a texture path. Applying a tint to the whole texture would also
tint white decals, glass, and other non-paint regions, so the renderer needs to
identify body-paint texels.

## Decision

1. The web car catalog is the opt-in source.
   - `CarOption` gains optional `paintable` and `defaultPaint` fields.
   - Only the Deckard is opted in initially: `paintable: true` and
     `defaultPaint: 0` (Red).
   - Other textured cars remain factory livery until they are explicitly
     opted in.

2. The renderer gains an opt-in textured-body paint path.
   - A textured body is paint-capable when at least 10% of its body texture
     looks like red paint, using the existing red-dominance heuristic.
   - Recolouring changes only paint-classified texels and preserves relative
     luminance and alpha. Non-paint texels stay byte-identical.
   - The painted body uses a cloned body texture. The original texture stays
     on the brake/reverse light meshes, and interior, glass, wheels, and other
     materials are untouched.
   - A paintable textured bay reports `recolorable: true`; non-paintable
     textured bays keep `recolorable: false` and "Factory livery".
   - The hint is threaded through the Garage and Drive viewer APIs. Only the
     local Drive slot may receive a textured paint; AI and remote slots that
     reuse the same car visual keep the original texture.

3. Paint remains visual-only and local.
   - The garage keeps storing a palette index per car id in
     `lapster.garage.paint.v1`.
   - The Garage Drive link carries the selected paint as `paint=<index>`.
   - Drive applies the textured paint only when an explicit paint value is
     present; launching without one keeps the factory texture.
   - No schema, ABI, protocol, or physics change.

4. Verification covers the mechanism and the rollout boundary.
   - Render unit tests cover texture classification, selective recolouring,
     garage bay paintability, and the Drive local-slot hint.
   - Browser tests cover the Deckard default Red, Blue persistence, the Drive
     `paint` link, `texturePaint` telemetry, and the Cavallo factory-livery
     behaviour.

## Consequences

### Positive

- The Deckard uses the same picker and storage as the Mazda.
- Other textured cars keep their existing factory livery without code changes.
- Brake/reverse lights and other non-body materials are not repainted.
- There is no asset-schema or network-contract change.

### Negative / risks

- Red-dominant livery or decal texels on an opted-in car are repainted with the
  body colour. This is acceptable for the Deckard's minimal livery but must be
  reviewed before opting in busier cars.
- A painted textured body keeps an extra CPU texture copy and uploads a new
  body texture on each selection. The Deckard body is ~4 MB at 1024² RGBA.
- The paint-capability decision is a render heuristic plus a web opt-in; future
  cars need both. A full authored paint mask would be more precise.

## Alternatives considered

- **Tint the whole textured body through vertex colour.** Rejected: it also
  tints decals and non-paint texture regions.
- **Enable every textured body automatically.** Rejected: the Cavallo and 67GP
  cars have busier liveries and should stay factory until reviewed.
- **Bake a paint mask or split paint geometry in the asset pack.** Deferred:
  more precise, but requires an asset-schema or pack-format change, an ADR for
  the contract update, and a fixture rebuild.
- **Keep the Deckard on factory livery.** Rejected by the product owner.

## Follow-up

- Validate the Deckard palette on the dev preview, especially red decals and
  the brake/reverse lenses.
- If more cars or busier liveries need paint, design an authored paint mask
  instead of widening the red-dominance heuristic.
- Consider moving the opt-in flag into pack metadata if the catalog stops being
  the authoritative car list.

## Amendment (2026-11-25): palette paint for AI and remote slots

The ten-car grid made nine identical factory-red Deckard AI cars visible, and
the product owner asked for the same palette variety the vertex-painted cars
already have. This reverses the local-only part of decision 2.

- A paintable textured car now applies the paint hint to every Drive slot, not
  only the local slot.
- `color_offset` remains the selected paint when one is supplied, otherwise 0.
  Bay `i` therefore uses `BAY_COLORS[(color_offset + i) % 10]`: with an explicit
  paint a ten-car field uses all ten palette colours exactly once; without one
  the local slot stays factory livery and AI slots use palette indices 1..9.
- The renderer reuses the existing per-slot cloned body texture. A full painted
  ten-car Deckard field can retain ten body-texture copies (~40 MB at 1024²
  RGBA) plus their uploads; this is accepted for the grid. TD-0024 tracks the
  memory cost and the eventual authored or shader paint mask.
- Non-paintable textured cars are unchanged and keep factory livery; they have
  not been reviewed for the red-dominance heuristic.
