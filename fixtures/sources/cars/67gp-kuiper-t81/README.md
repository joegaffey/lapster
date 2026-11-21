# Kuiper T81 — Speed Dreams source

This directory holds the tracked source files for Lapster's Speed Dreams 67GP
car import. The baked runtime pack is `fixtures/cars/67gp-kuiper-t81/`.

## Attribution

- **Title:** Kuiper T81
- **Author:** Copyright 2017 CHAD (readme); XML copyright (C) 2018 C Phillips.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-kuiper-t81
- **Source commit:** `141cc7a922238b3772c15dc4bfbd7d4eb4872f2d`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Local license copy:** `LICENSE-FAL-1.3.txt`
- **Downloaded:** 2026-11-21

Required credit line:

> Kuiper T81 by CHAD and C Phillips, Free Art License 1.3, via Speed Dreams.

FAL requires modified copies to remain under FAL or a compatible license and to
identify the modifications. Lapster's modifications are listed below.

This attribution covers copyright only. Kuiper and related marks belong to their respective owners. Lapster does not imply
manufacturer affiliation.

## Files

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `body/67gp-kuiper-t81.acc` | Speed Dreams body source | `3620d964234a8b5545bf77641af6ae6550dd006e20e10159e21a50a113f945dc` |
| `body/67gp-kuiper-t81.xml` | Car specification, physics, sounds, lights | `a7e20e5b1177fe6f4f6b4376899b42dbdccc614eb0b01cc1e307cfda0215ed94` |
| `body/67gp-kuiper-t81.png` | Exterior skin texture | `b8e22048f8e23787667e607d2b234d1a4f0f61f78a0eb426856756c2b38294be` |
| `body/readme.txt` | Original author/license note | `c063c3567554077eb6f9880c6707d204978a72fc182f95ba2519f52732e7b458` |
| `wheel/wheel0.acc` | Speed Dreams wheel source (single model, all corners) | `45a7e10ac61d87a88d2da59bb30191f9ed07b8fde158ee170e1d2b16964b7015` |
| `wheel/wheel3d.png` | Wheel texture | `ab2891ecd7049f79befc59145438b4f0346f006452c973428dd51fa81874c379` |
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
- Raised the body by 0.057 m as the initial suspension ride height,
  derived from the source rear brake-disc/hub geometry (`BRAKES_s_11` rear brake-disc geometry local
  y ~= 0.2740 m; rear wheel radius 0.3309 m; unrounded lift
  ~= 0.0569 m). Visual calibration is deferred to the render/web
  integration batch.
- Assigned the transparent `Glass` material to `WIGLASS1_s_2`, `WIGLASS2_s_0`, `WIGLASS3_s_0` (pattern `^WIGLASS`).
- Used a single wheel model (`wheel0`) at all four corners; the front-right
  and rear-right wheels get the 180-degree Y rotation.
- Imported the body and wheel textures.
- Mapped the source XML into runtime `CarPhysics` mass, engine, gearbox, brakes,
  steering, suspension, tire mu, and aero.

For the conversion and remaining work, see
[`docs/speed-dreams-pipeline.md`](../../../../docs/speed-dreams-pipeline.md).
