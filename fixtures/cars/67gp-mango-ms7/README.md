# Mango MS7 — Speed Dreams car

Textured Speed Dreams 67GP car prepared for Lapster.

## Attribution

- **Title:** Mango MS7
- **Author:** Copyright 2019 CHAD (readme); XML copyright (C) 2022 C Phillips.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-mango-ms7
- **Source commit:** `5ef9f5c3555a375d3002cc6fe55d40f1303d7e3d`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Source fixture:** `fixtures/sources/cars/67gp-mango-ms7/`

Required credit line:

> Mango MS7 by CHAD and C Phillips, Free Art License 1.3, via Speed Dreams.

Mango and related marks belong to their respective owners. Lapster does not imply manufacturer affiliation.

## Files

- `67gp-mango-ms7.sima`: baked pack.
- Source files are tracked separately in `fixtures/sources/cars/67gp-mango-ms7/`.

## Pack

- SHA-256: `07435078735f756ac4778535d12bdc119852b81e644cbaea04f60cfb5943eb7d`
- Byte size: 5,092,956 bytes.
- ID: `67gp-mango-ms7`
- Schema: 3
- 5,571 vertices, 12,540 triangles, 3 materials,
  2 textures, 6 draw calls, 6 meshes.
- Textured body, glass, and a single wheel model reused at all four corners.
  Body glass nodes use the shared transparent `Glass` material.
- Source XML mapped into runtime physics: 590 kg, 0.23 m CG height, 42/58 weight distribution, 21-point torque curve (1,000 rpm idle, 10,000 rpm redline), five-speed RWD gearbox (2.66 / 1.78 / 1.35 / 1.01 / 0.86) with a 3.73 final drive, 23° steering lock, 35/19 kN/m spring rates, 30/5 kN/m anti-roll bars, 0.53 brake bias, tire mu 1.2, and aerodynamic coefficients.
- 2.30 m wheelbase, 1.50 m front track, and
  1.52 m rear track. Front/rear wheel radii are
  0.3089/0.3390 m; initial body lift is 0.130 m
  (uncalibrated).

## Rebuild

Install the tools described in `tools/speed-dreams/README.md`, then:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-mango-ms7 \
  fixtures/cars/67gp-mango-ms7/67gp-mango-ms7.sima
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/67gp-mango-ms7/67gp-mango-ms7.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-mango-ms7/67gp-mango-ms7.sima
```

See [`docs/speed-dreams-pipeline.md`](../../../docs/speed-dreams-pipeline.md) for
the full import process, assumptions, and remaining work.
