# Murasama 37A — Speed Dreams car

Textured Speed Dreams 67GP car prepared for Lapster.

## Attribution

- **Title:** Murasama 37A
- **Author:** Copyright 2022 WEC (readme); XML header carries an inherited (C) 2016 by WEC notice.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-murasama-37a
- **Source commit:** `1fe116f1bdec85abc3fd5a1b45b6aafac877f01b`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Source fixture:** `fixtures/sources/cars/67gp-murasama-37a/`

Required credit line:

> Murasama 37A by WEC, Free Art License 1.3, via Speed Dreams.

Murasama and related marks belong to their respective owners. Lapster does not imply manufacturer affiliation.

## Files

- `67gp-murasama-37a.sima`: baked pack.
- Source files are tracked separately in `fixtures/sources/cars/67gp-murasama-37a/`.

## Pack

- SHA-256: `866a7b4c859c82180b1e5341146452da6313d0bee3cf5ec19c56780fa206f93d`
- Byte size: 3,361,216 bytes.
- ID: `67gp-murasama-37a`
- Schema: 3
- 11,448 vertices, 19,355 triangles, 4 materials,
  3 textures, 7 draw calls, 7 meshes.
- Textured body, glass, and a single wheel model reused at all four corners.
  Body glass nodes use the shared transparent `Glass` material.
- Source XML mapped into runtime physics: 690 kg, 0.245 m CG height, 41/59 weight distribution, 42-point torque curve (1,000 rpm idle, 10,000 rpm redline), five-speed RWD gearbox (3.50 / 2.40 / 1.795 / 1.224 / 0.97) with a 3.73 final drive, 30° steering lock, 35/19 kN/m spring rates, 30/5 kN/m anti-roll bars, 0.53 brake bias, tire mu 1.2, and aerodynamic coefficients.
- 2.72 m wheelbase, 1.50 m front track, and
  1.46 m rear track. Front/rear wheel radii are
  0.2974/0.3399 m; initial body lift is 0.117 m
  (uncalibrated).

## Rebuild

Install the tools described in `tools/speed-dreams/README.md`, then:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-murasama-37a \
  fixtures/cars/67gp-murasama-37a/67gp-murasama-37a.sima
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/67gp-murasama-37a/67gp-murasama-37a.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-murasama-37a/67gp-murasama-37a.sima
```

See [`docs/speed-dreams-pipeline.md`](../../../docs/speed-dreams-pipeline.md) for
the full import process, assumptions, and remaining work.
