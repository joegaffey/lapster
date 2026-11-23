# Upping UT19 — Speed Dreams source

This directory holds the tracked source files for Lapster's Speed Dreams 67GP
car import. The baked runtime pack is `fixtures/cars/67gp-upping-ut19/`.

## Attribution

- **Title:** Upping UT19
- **Author:** Copyright 2015 CHAD (readme); XML copyright (C) 2015 C Phillips.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-upping-ut19
- **Source commit:** `12de53e34d2d8fdf7c41b5185dfb734ff8c67240`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Local license copy:** `LICENSE-FAL-1.3.txt`
- **Downloaded:** 2026-11-21

Required credit line:

> Upping UT19 by CHAD and C Phillips, Free Art License 1.3, via Speed Dreams.

FAL requires modified copies to remain under FAL or a compatible license and to
identify the modifications. Lapster's modifications are listed below.

This attribution covers copyright only. Upping and related marks belong to their respective owners. Lapster does not imply
manufacturer affiliation.

## Files

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `body/67gp-upping-ut19.acc` | Speed Dreams body source | `3dea76017e67ba349c6d5cf0344f0767d46c9c2a7f7d4eba49bb3d7dec1fe7ef` |
| `body/67gp-upping-ut19.xml` | Car specification, physics, sounds, lights | `d95bfd3986348c9ed04dd5d2caa5472caf2a129337e24c638cd039248f71867d` |
| `body/67gp-upping-ut19.png` | Exterior skin texture | `cebc6bffdda79d85c4bdd94ab7d2c7ab6dc22f62ba1c959f1767ed2aefaa6d31` |
| `body/readme.txt` | Original author/license note | `e435ec53654892ade5184aad7efd58e945bb1a090bb80a1b8679cc0466647d2f` |
| `wheel/wheel0.acc` | Speed Dreams wheel source (single model, all corners) | `45a7e10ac61d87a88d2da59bb30191f9ed07b8fde158ee170e1d2b16964b7015` |
| `wheel/wheel3d.png` | Wheel texture | `6fe7c16b237dfcc22e1d1d9eeff854459add2d26bfc7419b20ed86c0fe1b2a0b` |
| `LICENSE-FAL-1.3.txt` | Free Art License text | `0b1e1674f969b4389ecf9429bde45768ce91d0ecb4af35f360e87faceafe6879` |

`import.json` is Lapster-authored build configuration; it is not part of the
upstream source package. The body references only the exterior skin texture. Auxiliary Speed Dreams files that are
not needed by the current import — alternate skins, driver models,
steering-wheel models, gauge textures, shadow texture, and sound samples —
remain available in the upstream repository at the commit above.

## Provenance

All retained files were copied from the upstream repository at the pinned
commit and verified byte-for-byte by SHA-256 against the upstream Git blobs.
The upstream repository does not carry a separate license file; the local
`LICENSE-FAL-1.3.txt` is the shared Lapster copy of the FAL 1.3 text, and the
upstream `readme.txt` and XML header both state FAL/copyleft terms.

## Modifications by Lapster

- Pruned the source package to the body, body texture(s), XML, readme, one
  wheel model, and wheel texture. Driver, steering-wheel, audio, XCF, `.ac`
  source, shadow, and unused `wheel1..3` files were not copied.
- Exported the `.acc` body and wheel to glTF with Assimp 6.0.4.
- Rotated the body from Speed Dreams `+X` forward into Lapster `-Z` forward.
- Raised the body by 0.043 m as the initial suspension ride height,
  derived from the source rear brake-disc/hub geometry (`BRAKES_s_17` rear brake-disc geometry local
  y ~= 0.2974 m; rear wheel radius 0.3399 m; unrounded lift
  ~= 0.0425 m). Visual calibration is deferred to the render/web
  integration batch.
- Assigned the transparent `Glass` material to `WIGLASS_s_4` (pattern `^WIGLASS`).
- Used a single wheel model (`wheel0`) at all four corners; the front-right
  and rear-right wheels get the 180-degree Y rotation.
- Imported the body and wheel textures.
- Mapped the source XML into runtime `CarPhysics` mass, engine, gearbox, brakes,
  steering, suspension, tire mu, and aero.

For the conversion and remaining work, see
[`docs/speed-dreams-pipeline.md`](../../../../docs/speed-dreams-pipeline.md).
