# Cavallo 123 — Speed Dreams source

This directory holds the tracked source files for Lapster's Speed Dreams 67GP
car import. The baked runtime pack is `fixtures/cars/67gp-cavallo-123/`.

## Attribution

- **Title:** Cavallo 123
- **Author:** W-E-C / WEC (original model, via txChange); physics and setup by Xavier Bertaux (2018). XML copyright (C) 2016 by WEC.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-cavallo-123
- **Source commit:** `edf33a67807f579fc3bc3aa341c8b3c4fa22489c`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Local license copy:** `LICENSE-FAL-1.3.txt`
- **Downloaded:** 2026-11-21

Required credit line:

> Cavallo 123 by W-E-C / WEC with physics and setup by Xavier Bertaux, Free Art License 1.3, via Speed Dreams.

FAL requires modified copies to remain under FAL or a compatible license and to
identify the modifications. Lapster's modifications are listed below.

This attribution covers copyright only. Cavallo and related marks belong to their respective owners. Lapster does not imply
manufacturer affiliation.

## Files

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `body/67gp-cavallo-123.acc` | Speed Dreams body source | `2fcd81a73ea4486987e4438e2c8c0e86ad0366274711fce3116a02dc8d03e385` |
| `body/67gp-cavallo-123.xml` | Car specification, physics, sounds, lights | `5361fe3e43a19d3303e72aec8cdce7a5c90e38b0fa089eaf3b796609a320a4ea` |
| `body/67gp-cavallo-123.png` | Exterior skin texture | `8a2672eca473940d9c21e963155fa986cd02c5aaae722468848d047cf1cfc870` |
| `body/67gp-cavallo-123-int.png` | Interior texture referenced by the body | `4a7fa9f892bee856e60e1a6fa1c0c4ff583563631bb50122448eae2ac7f3e8e2` |
| `body/readme.txt` | Original author/license note | `89f755f5eca356ad2daddd8e8cbc469f9a6745a94a39e20d25e178fb78135e6f` |
| `wheel/wheel0.acc` | Speed Dreams wheel source (single model, all corners) | `78c09ca6006ff0f1225dbf4ee90731f1156e39f8a419e2b970dc9db0d3798dbb` |
| `wheel/wheel3d.png` | Wheel texture | `064516eed6fc688f7f0cb3cecaa8d146c43a0c24ad24a9268e2f4d1d194401cf` |
| `LICENSE-FAL-1.3.txt` | Free Art License text | `0b1e1674f969b4389ecf9429bde45768ce91d0ecb4af35f360e87faceafe6879` |

`import.json` is Lapster-authored build configuration; it is not part of the
upstream source package. The body references an interior texture, so `body/67gp-cavallo-123-int.png` is retained in addition to the exterior skin. Auxiliary Speed Dreams files that are
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
- Raised the body by 0.076 m as the initial suspension ride height,
  derived from the source rear brake-disc/hub geometry (`RAXLE_s_3` rear axle hub local
  y ~= 0.2636 m; rear wheel radius 0.3399 m; unrounded lift
  ~= 0.0763 m). Visual calibration is deferred to the render/web
  integration batch.
- Assigned the transparent `Glass` material to `WIFRONT_s_0` (pattern `^WIFRONT`).
- Excluded the body node `WHEELARCH_s_1` via `exclude_nodes`, because its name would otherwise be counted as a fifth wheel node by the pack baker.
- Used a single wheel model (`wheel0`) at all four corners; the front-right
  and rear-right wheels get the 180-degree Y rotation.
- Imported the body and wheel textures.
- Mapped the source XML into runtime `CarPhysics` mass, engine, gearbox, brakes,
  steering, suspension, tire mu, and aero.

For the conversion and remaining work, see
[`docs/speed-dreams-pipeline.md`](../../../../docs/speed-dreams-pipeline.md).
