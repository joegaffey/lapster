# Cavallo 123 — Speed Dreams car

Textured Speed Dreams 67GP car prepared for Lapster.

## Attribution

- **Title:** Cavallo 123
- **Author:** W-E-C / WEC (original model, via txChange); physics and setup by Xavier Bertaux (2018). XML copyright (C) 2016 by WEC.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-cavallo-123
- **Source commit:** `edf33a67807f579fc3bc3aa341c8b3c4fa22489c`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Source fixture:** `fixtures/sources/cars/67gp-cavallo-123/`

Required credit line:

> Cavallo 123 by W-E-C / WEC with physics and setup by Xavier Bertaux, Free Art License 1.3, via Speed Dreams.

Cavallo and related marks belong to their respective owners. Lapster does not imply manufacturer affiliation.

## Files

- `67gp-cavallo-123.sima`: baked pack.
- Source files are tracked separately in `fixtures/sources/cars/67gp-cavallo-123/`.

## Pack

- SHA-256: `c5d422496b08b412dc74de30c98f9167d68eb03a712d8e7033d2500cce76ce13`
- Byte size: 3,165,936 bytes.
- ID: `67gp-cavallo-123`
- Schema: 3
- 8,520 vertices, 13,823 triangles, 4 materials,
  3 textures, 7 draw calls, 7 meshes.
- Textured body, glass, and a single wheel model reused at all four corners.
  Body glass nodes use the shared transparent `Glass` material.
- Source XML mapped into runtime physics: 628 kg, 0.215 m CG height, 44/56 weight distribution, 42-point torque curve (1,000 rpm idle, 10,000 rpm redline), five-speed RWD gearbox (3.50 / 2.40 / 1.795 / 1.224 / 0.95) with a 3.73 final drive, 30° steering lock, 35/19 kN/m spring rates, 30/5 kN/m anti-roll bars, 0.51 brake bias, tire mu 1.2, and aerodynamic coefficients.
- 2.36 m wheelbase, 1.50 m front track, and
  1.46 m rear track. Front/rear wheel radii are
  0.2974/0.3399 m; initial body lift is 0.076 m
  (uncalibrated).

## Rebuild

Install the tools described in `tools/speed-dreams/README.md`, then:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-cavallo-123 \
  fixtures/cars/67gp-cavallo-123/67gp-cavallo-123.sima
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/67gp-cavallo-123/67gp-cavallo-123.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-cavallo-123/67gp-cavallo-123.sima
```

See [`docs/speed-dreams-pipeline.md`](../../../docs/speed-dreams-pipeline.md) for
the full import process, assumptions, and remaining work.
