# Guthrie Eagle 67GP — Speed Dreams car

Textured Speed Dreams 67GP car prepared for Lapster.

## Attribution

- **Title:** Guthrie Eagle 67GP
- **Author:** Copyright 2018 CHAD (Chad Phillips)
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-guthrie-eagle
- **Source commit:** `01c82922ba16697e1a7d4766013ebadea26bd72c`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Source fixture:** `fixtures/sources/cars/67gp-guthrie-eagle/`

Required credit line:

> Guthrie Eagle 67GP by CHAD (Chad Phillips), Free Art License 1.3, via Speed
> Dreams.

Eagle and related marks belong to their respective owners. Lapster does not
imply manufacturer affiliation.

## Files

- `67gp-guthrie-eagle.sima`: baked pack.
- Source files are tracked separately in `fixtures/sources/cars/67gp-guthrie-eagle/`.

## Pack

- SHA-256: `9bb0c85d17084e22bc7902a0c03c6410efa99d145b7f7ddf69ae4505b0b86921`
- Byte size: 5,329,844 bytes.
- ID: `67gp-guthrie-eagle`
- Schema: 3
- 9,105 vertices, 19,328 triangles, 3 materials, 2 textures, 6 draw calls.
- Textured body, glass, and a single wheel model reused at all four corners.
  Body glass nodes use the shared transparent `Glass` material.
- Source XML mapped into runtime physics: 620 kg, 0.19 m CG height, 45/55
  weight distribution, 3.0 L V12 with a 14-point torque curve (1,000 rpm idle,
  9,500 rpm redline), five-speed RWD gearbox with a 3.73 final drive, 30°
  steering lock, brake pressure/areas, spring/damper rates, anti-roll bars, tire
  mu 1.2, and aerodynamic coefficients.
- 2.46 m wheelbase, 1.54 m front track, and 1.58 m rear track.

## Rebuild

Install the tools described in `tools/speed-dreams/README.md`, then:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/67gp-guthrie-eagle \
  fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
cargo run -p asset-compiler -- validate fixtures/cars/67gp-guthrie-eagle/67gp-guthrie-eagle.sima
```

See [`docs/speed-dreams-pipeline.md`](../../../docs/speed-dreams-pipeline.md) for
the full import process, assumptions, and remaining work.
