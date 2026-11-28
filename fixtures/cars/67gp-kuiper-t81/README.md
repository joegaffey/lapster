# Kuiper T81 — Speed Dreams car

Textured Speed Dreams 67GP car prepared for Lapster.

## Attribution

- **Title:** Kuiper T81
- **Author:** Copyright 2017 CHAD (readme); XML copyright (C) 2018 C Phillips.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-kuiper-t81
- **Source commit:** `141cc7a922238b3772c15dc4bfbd7d4eb4872f2d`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Source fixture:** `fixtures/sources/cars/67gp-kuiper-t81/`

Required credit line:

> Kuiper T81 by CHAD and C Phillips, Free Art License 1.3, via Speed Dreams.

Kuiper and related marks belong to their respective owners. Lapster does not imply manufacturer affiliation.

## Files

- `67gp-kuiper-t81.sima`: baked pack.
- Source files are tracked separately in `fixtures/sources/cars/67gp-kuiper-t81/`.

## Pack

- SHA-256: `eec94fc1d6b2b6278d58e250d948e7424430b90d7969c17a7559bdd4d7e18b82`
- Byte size: 5,291,652 bytes.
- ID: `67gp-kuiper-t81`
- Schema: 3
- 8,291 vertices, 19,114 triangles, 3 materials,
  2 textures, 6 draw calls, 6 meshes.
- Textured body, glass, and a single wheel model reused at all four corners.
  Body glass nodes use the shared transparent `Glass` material.
- Source XML mapped into runtime physics: 690 kg, 0.225 m CG height, 42/58 weight distribution, 42-point torque curve (1,000 rpm idle, 10,000 rpm redline), five-speed RWD gearbox (2.66 / 1.78 / 1.35 / 1.01 / 0.87) with a 3.73 final drive, 30° steering lock, 35/19 kN/m spring rates, 30/10 kN/m anti-roll bars, 0.56 brake bias, tire mu 1.2, and aerodynamic coefficients.
- 2.48 m wheelbase, 1.46 m front track, and
  1.48 m rear track. Front/rear wheel radii are
  0.3101/0.3309 m; initial body lift is 0.057 m
  (uncalibrated).

## Rebuild

Install the tools described in `tools/speed-dreams/README.md`, then:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-kuiper-t81 \
  fixtures/cars/67gp-kuiper-t81/67gp-kuiper-t81.sima
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/67gp-kuiper-t81/67gp-kuiper-t81.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-kuiper-t81/67gp-kuiper-t81.sima
```

See [`docs/speed-dreams-pipeline.md`](../../../docs/speed-dreams-pipeline.md) for
the full import process, assumptions, and remaining work.
