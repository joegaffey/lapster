# Mazda RX-7 — Default Demo Car

## Attribution

- **Title:** Mazda RX-7
- **Creator:** IvOfficial
- **Source:** https://poly.pizza/m/SnIoWlh7S2
- **License:** Creative Commons Attribution 3.0 Unported (CC BY 3.0)
- **License URL:** https://creativecommons.org/licenses/by/3.0/
- **Downloaded:** 2026-10-23

Required credit line:

> Mazda RX-7 by IvOfficial, CC BY 3.0, via Poly Pizza.

CC BY covers copyright only. Mazda and RX-7 are trademarks of their respective owners. The
import removes the source `Logos` mesh; do not imply Mazda affiliation.

## Files

- `mazda-rx7.glb`: source model.
- `mazda-rx7.sima`: baked pack used by the garage and drive demos.

Rebuild the pack:

```bash
cargo run -p asset-compiler -- bake-glb fixtures/cars/mazda-rx7/mazda-rx7.glb
```

The import applies node transforms, rotates the car to `-Z` forward, extracts four wheels,
removes logo geometry, bakes the palette texture into vertex colours, and uses generic
rear-wheel-drive physics.

## Geometry

- 35,132 vertices, 18,322 triangles, 5 draw calls.
- Body: 4,898 triangles; wheels: 4 × 3,294; exhaust: 248.
- No steering wheel or interior.
