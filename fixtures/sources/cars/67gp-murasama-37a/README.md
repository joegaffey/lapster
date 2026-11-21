# Murasama 37A — Speed Dreams source

This directory holds the tracked source files for Lapster's Speed Dreams 67GP
car import. The baked runtime pack is `fixtures/cars/67gp-murasama-37a/`.

## Attribution

- **Title:** Murasama 37A
- **Author:** Copyright 2022 WEC (readme); XML header carries an inherited (C) 2016 by WEC notice.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-murasama-37a
- **Source commit:** `1fe116f1bdec85abc3fd5a1b45b6aafac877f01b`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Local license copy:** `LICENSE-FAL-1.3.txt`
- **Downloaded:** 2026-11-21

Required credit line:

> Murasama 37A by WEC, Free Art License 1.3, via Speed Dreams.

FAL requires modified copies to remain under FAL or a compatible license and to
identify the modifications. Lapster's modifications are listed below.

This attribution covers copyright only. Murasama and related marks belong to their respective owners. Lapster does not imply
manufacturer affiliation.

## Files

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `body/67gp-murasama-37a.acc` | Speed Dreams body source | `1a79a33e809bbe9f803a3ec34c885c64d1c773b236353bec6aa2c72b9df53dbc` |
| `body/67gp-murasama-37a.xml` | Car specification, physics, sounds, lights | `2e249556be568f90322b5f6869cac1c8093261eec20e7930b1fc037ccb6b31bc` |
| `body/67gp-murasama-37a.png` | Exterior skin texture | `6e15c2ecbbf447f7f041fae2a0c8119b538161cb9850523d6f9aec7d1642981b` |
| `body/67gp-murasama-37a-int.png` | Interior texture referenced by the body | `7ff2495d3cec3350acc9575661750dcad9d6f8a1a916aa88b8c0c83250a70fbb` |
| `body/readme.txt` | Original author/license note | `c203daa6b76be3e0660a35f043da293178d97d1187ff98d0054567b7f43d3e54` |
| `wheel/wheel0.acc` | Speed Dreams wheel source (single model, all corners) | `78c09ca6006ff0f1225dbf4ee90731f1156e39f8a419e2b970dc9db0d3798dbb` |
| `wheel/wheel3d.png` | Wheel texture | `aea1bb037109ccd51055a00c893f3aa2160df384c8a6e95e4b04fbd5a0c4f558` |
| `LICENSE-FAL-1.3.txt` | Free Art License text | `0b1e1674f969b4389ecf9429bde45768ce91d0ecb4af35f360e87faceafe6879` |

`import.json` is Lapster-authored build configuration; it is not part of the
upstream source package. The body references an interior texture, so `body/67gp-murasama-37a-int.png` is retained in addition to the exterior skin. Auxiliary Speed Dreams files that are
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
- Raised the body by 0.117 m as the initial suspension ride height,
  derived from the source rear brake-disc/hub geometry (`RAXLEMAT1_s_19` rear axle/brake-disc geometry local
  y ~= 0.2225 m; rear wheel radius 0.3399 m; unrounded lift
  ~= 0.1174 m). Visual calibration is deferred to the render/web
  integration batch.
- Assigned the transparent `Glass` material to `WI1MAT0_s_1`, `WI2MAT0_s_0`, `SCREEN2MAT0_s_5` (patterns `^WI[12]MAT0`, `^SCREEN2MAT0`).
- Used a single wheel model (`wheel0`) at all four corners; the front-right
  and rear-right wheels get the 180-degree Y rotation.
- Imported the body and wheel textures.
- Mapped the source XML into runtime `CarPhysics` mass, engine, gearbox, brakes,
  steering, suspension, tire mu, and aero.

For the conversion and remaining work, see
[`docs/speed-dreams-pipeline.md`](../../../../docs/speed-dreams-pipeline.md).
