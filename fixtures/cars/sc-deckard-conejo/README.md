# Deckard Conejo RR — Speed Dreams demo car

Textured Speed Dreams car used by the Drive and Garage demos.

## Attribution

- **Title:** Deckard Conejo RR (2010 Chevrolet Camaro SS)
- **Authors:** Tom_D3, W.E.C, Chad Phillips, and June Ravenmoon
- **Source:** https://forge.a-lec.org/speed-dreams/sc-deckard-conejo
- **Source commit:** `ba52857eac4d415510685403e06263de67bffee9`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Wheel donor:** Boxer 96 Carson Tuners Edition, Speed Dreams, FAL 1.3
- **Source fixture:** `fixtures/sources/cars/sc-deckard-conejo/`

Required credit line:

> Deckard Conejo RR by Tom_D3, W.E.C, Chad Phillips, and June Ravenmoon,
> Free Art License 1.3, via Speed Dreams. Wheel from Boxer 96 Carson Tuners
> Edition by Eric Espie, Jean-Christophe Durieu, Olaf Sassnick, Bernhard
> Wymann, Jean-Philippe Meuret, W.E.C, Eckhard M. Jäger, and Chad Phillips.

Chevrolet, Camaro, Porsche, and 911/996 are trademarks of their respective
owners. Lapster does not imply manufacturer affiliation.

## Files

- `sc-deckard-conejo.sima`: baked pack used by the menu, Garage, and Drive.
- Source files are tracked separately in `fixtures/sources/cars/sc-deckard-conejo/`.

## Pack

- SHA-256: `4812013118dd62ac459391c4a65660265963b12d772cfab8ac050448dbac8b11`
- ID: `sc-deckard-conejo`
- Schema: 3
- 11,352 vertices, 13,058 triangles, 4 materials, 3 textures, 7 draw calls.
- Textured body + interior, glass, four donor wheels, brake/reverse lamp meshes.
- Body/interior are split so cockpit mode can use the interior material without
  exterior outlines.
- Source XML mapped into runtime physics: 1,700 kg, 0.30 m CG height, 52/48 weight
  distribution, 6.2 L V8 with a 21-point torque curve, six-speed RWD gearbox with
  3.8 final drive, source steering lock, brake pressure/areas, spring/damper rates,
  anti-roll bars, tire mu, and aerodynamic coefficients.
- 2.73 m wheelbase, 1.62 m track; 6,426,968 bytes.

## Rebuild

Install the tools described in `tools/speed-dreams/README.md`, then:

```bash
tools/speed-dreams/build-car.sh \
  fixtures/sources/cars/sc-deckard-conejo \
  fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
```

Validate:

```bash
cargo run -p asset-compiler -- inspect fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
cargo run -p asset-compiler -- validate fixtures/cars/sc-deckard-conejo/sc-deckard-conejo.sima
```

See [`docs/speed-dreams-pipeline.md`](../../../docs/speed-dreams-pipeline.md) for
the full import process, assumptions, and remaining work.
