# Upping UT19 — Speed Dreams car

Textured Speed Dreams 67GP car prepared for Lapster.

## Attribution

- **Title:** Upping UT19
- **Author:** Copyright 2015 CHAD (readme); XML copyright (C) 2015 C Phillips.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-upping-ut19
- **Source commit:** `12de53e34d2d8fdf7c41b5185dfb734ff8c67240`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Source fixture:** `fixtures/sources/cars/67gp-upping-ut19/`

Required credit line:

> Upping UT19 by CHAD and C Phillips, Free Art License 1.3, via Speed Dreams.

Upping and related marks belong to their respective owners. Lapster does not imply manufacturer affiliation.

## Files

- `67gp-upping-ut19.sima`: baked pack.
- Source files are tracked separately in `fixtures/sources/cars/67gp-upping-ut19/`.

## Pack

- SHA-256: `cbb95ee516302ab04e8d8046b106fee1f51b7f9d607b0350e199d426083677ad`
- Byte size: 5,258,708 bytes.
- ID: `67gp-upping-ut19`
- Schema: 3
- 7,950 vertices, 17,619 triangles, 3 materials,
  2 textures, 6 draw calls, 6 meshes.
- Textured body, glass, and a single wheel model reused at all four corners.
  Body glass nodes use the shared transparent `Glass` material.
- Source XML mapped into runtime physics: 640 kg, 0.23 m CG height, 37/63 weight distribution, 37-point torque curve (1,000 rpm idle, 9,000 rpm redline), five-speed RWD gearbox (2.66 / 1.78 / 1.35 / 1.01 / 0.87) with a 3.73 final drive, 30° steering lock, 30/20 kN/m spring rates, 20/5 kN/m anti-roll bars, 0.52 brake bias, tire mu 1.2, and aerodynamic coefficients.
- 2.60 m wheelbase, 1.50 m front track, and
  1.46 m rear track. Front/rear wheel radii are
  0.3101/0.3399 m; initial body lift is 0.043 m
  (uncalibrated).

## Rebuild

Install the tools described in `tools/speed-dreams/README.md`, then:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-upping-ut19 \
  fixtures/cars/67gp-upping-ut19/67gp-upping-ut19.sima
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/67gp-upping-ut19/67gp-upping-ut19.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-upping-ut19/67gp-upping-ut19.sima
```

See [`docs/speed-dreams-pipeline.md`](../../../docs/speed-dreams-pipeline.md) for
the full import process, assumptions, and remaining work.
