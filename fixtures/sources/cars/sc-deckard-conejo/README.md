# Deckard Conejo RR — Speed Dreams source

This directory holds the tracked source files for Lapster's Speed Dreams car
import. The baked runtime pack is `fixtures/cars/sc-deckard-conejo/`.

## Attribution

- **Title:** Deckard Conejo RR (2010 Chevrolet Camaro SS)
- **Model author:** Tom_D3 (original model, February 2015)
- **2025 3D rework:** W.E.C
- **Interior parts:** Chad Phillips
- **Skins:** June Ravenmoon; 2025 skin rework by W.E.C
- **Source:** https://forge.a-lec.org/speed-dreams/sc-deckard-conejo
- **Source commit:** `ba52857eac4d415510685403e06263de67bffee9`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Local license copy:** `LICENSE-FAL-1.3.txt`
- **Downloaded:** 2026-11-01

Required credit line:

> Deckard Conejo RR by Tom_D3, W.E.C, Chad Phillips, and June Ravenmoon,
> Free Art License 1.3, via Speed Dreams.

The donor wheel is from the Speed Dreams **Boxer 96 Carson Tuners Edition**:

- **Source:** https://forge.a-lec.org/speed-dreams/sc-carson-96
- **Source commit:** `df202bcb9df324384a4ef209420c104e689b2246`
- **License:** Free Art License 1.3
- Copyright 2002 Eric Espie, Jean-Christophe Durieu; 2005 Olaf Sassnick,
  Bernhard Wymann; 2007 Bernhard Wymann; 2009 Jean-Philippe Meuret;
  2011, 2013, 2024 W.E.C; 2011 Eckhard M. Jäger; 2021 Chad Phillips.
- See `wheel/README.md` for the specific files and modifications.

FAL requires modified copies to remain under FAL or a compatible license and to
identify the modifications. Lapster's modifications are listed below.

This attribution covers copyright only. Chevrolet, Camaro, and related marks
belong to their respective owners. Lapster does not imply manufacturer
affiliation.

## Files

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `body/sc-deckard-conejo.acc` | Speed Dreams body source | `0e88b3ea2dda18be9ba7db32d3c28099b3c80da0e6b88b47296ad423f4b8332f` |
| `body/sc-deckard-conejo.xml` | Car specification, physics, sounds, lights | `c27955484e2a227303dd6e9904d9cea2bf9a8c1ceac69c0036b296843fa14ef1` |
| `body/sc-deckard-conejo.png` | Exterior skin texture | `400bb42a9aa5bf7ac713fa03f6023a9f632d84ed5af1ca173df3cdb228a1e96b` |
| `body/sc-deckard-conejo-int.png` | Interior texture | `dac8353822f1422e791b26f4ceed1c0a2c0c2e99af344a8f28925af32cd3aba0` |
| `body/sc-deckard-conejo-preview.jpg` | Source preview image | `de1731e2ea805415e720ca4f31b1c6a3d78d6f3030a875781cbe1e1f432ea970` |
| `body/readme.txt` | Original Speed Dreams readme | `c5ba5890934a22402ec417be1237cb0005e1b27264c909975f26167418168462` |
| `wheel/wheel0.acc` | Donor wheel source | `87d876ae86890de196687b7432630378b1e15a117ba5acb10c350d7ee36ebdb2` |
| `wheel/wheel3d.png` | Donor wheel texture | `9ca76bd2cc5bea9c07720c15554b77bb92b80c71c3ec0d3926bf4ba2da9728f9` |

Auxiliary Speed Dreams files that are not needed by the current import —
alternate skins, driver models, steering-wheel models, gauge textures, shadow
texture, and sound samples — remain available in the upstream repositories at
the commits above.

## Modifications by Lapster

- Exported the `.acc` body and donor wheel to glTF with Assimp.
- Rotated the body from Speed Dreams `+X` forward into Lapster `-Z` forward.
- Raised the body by 0.21 m to restore the source suspension ride height.
- Replaced the missing deckard wheel with the Boxer 96 Carson donor wheel.
- Split glass materials into the existing transparent-glass path.
- Imported the body and interior textures; disabled interior outlines.
- Baked brake and reverse lamp geometry for runtime emissive lighting.
- Used Lapster-generated gauge faces and the hardcoded Camaro cockpit layout
  instead of the source gauge textures.
- Used the Engine_Sim `v8cross` profile rather than the source `v8-ears68.wav`
  sample.
- Derived wheel positions and generic vehicle physics from the assembled GLB;
  full Speed Dreams XML physics mapping is still outstanding.

For the conversion and remaining work, see
[`docs/speed-dreams-pipeline.md`](../../../../docs/speed-dreams-pipeline.md).
