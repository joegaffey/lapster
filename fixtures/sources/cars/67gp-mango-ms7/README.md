# Mango MS7 — Speed Dreams source

This directory holds the tracked source files for Lapster's Speed Dreams 67GP
car import. The baked runtime pack is `fixtures/cars/67gp-mango-ms7/`.

## Attribution

- **Title:** Mango MS7
- **Author:** Copyright 2019 CHAD (readme); XML copyright (C) 2022 C Phillips.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-mango-ms7
- **Source commit:** `5ef9f5c3555a375d3002cc6fe55d40f1303d7e3d`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Local license copy:** `LICENSE-FAL-1.3.txt`
- **Downloaded:** 2026-11-21

Required credit line:

> Mango MS7 by CHAD and C Phillips, Free Art License 1.3, via Speed Dreams.

FAL requires modified copies to remain under FAL or a compatible license and to
identify the modifications. Lapster's modifications are listed below.

This attribution covers copyright only. Mango and related marks belong to their respective owners. Lapster does not imply
manufacturer affiliation.

## Files

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `body/67gp-mango-ms7.acc` | Speed Dreams body source | `8ab3f2976ad3121c3ee84f5408417be1a2b5157eb6944c921611db28f24e8131` |
| `body/67gp-mango-ms7.xml` | Car specification, physics, sounds, lights | `4650bc16a953a9b85c8a1e7b51f028e1f226a0bb0a315d7f689aae356b2f6050` |
| `body/67gp-mango-ms7.png` | Exterior skin texture | `30728a9708a7bad24310b68162e254968ab143d6d8d359d0da90247c7a50b94d` |
| `body/readme.txt` | Original author/license note | `eb91d44859831bcc3e5bed12b0897a58f6f58e67772a37bd10894ca97df97c9b` |
| `wheel/wheel0.acc` | Speed Dreams wheel source (single model, all corners) | `45a7e10ac61d87a88d2da59bb30191f9ed07b8fde158ee170e1d2b16964b7015` |
| `wheel/wheel3d.png` | Wheel texture | `3c33cecd3523f400ae2294f618ed602b66dc4706fad647f9bf2151102535418e` |
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
- Raised the body by 0.130 m as the initial suspension ride height,
  derived from the source rear brake-disc/hub geometry (`BRAKE1_s_9` / `DISK_s_7` rear brake-disc geometry local
  y ~= 0.2088 m; rear wheel radius 0.3390 m; unrounded lift
  ~= 0.1302 m). Visual calibration is deferred to the render/web
  integration batch.
- Assigned the transparent `Glass` material to `WI1_s_0`, `WI2_s_0`, `WI3_s_0` (pattern `^WI[123]_`).
- Used a single wheel model (`wheel0`) at all four corners; the front-right
  and rear-right wheels get the 180-degree Y rotation.
- Imported the body and wheel textures.
- Mapped the source XML into runtime `CarPhysics` mass, engine, gearbox, brakes,
  steering, suspension, tire mu, and aero.

For the conversion and remaining work, see
[`docs/speed-dreams-pipeline.md`](../../../../docs/speed-dreams-pipeline.md).
