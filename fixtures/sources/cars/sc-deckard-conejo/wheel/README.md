# Donor wheel — Speed Dreams Boxer 96 Carson Tuners Edition

The Deckard Conejo source package does not include wheel geometry, so the
Lapster import uses one wheel from the Speed Dreams Boxer 96 Carson Tuners
Edition.

## Attribution

- **Title:** Boxer 96 Carson Tuners Edition (SC Boxer 96 Carson Edition)
- **Source:** https://forge.a-lec.org/speed-dreams/sc-carson-96
- **Source commit:** `df202bcb9df324384a4ef209420c104e689b2246`
- **License:** Free Art License 1.3
- **License text:** [`../LICENSE-FAL-1.3.txt`](../LICENSE-FAL-1.3.txt)
- **Copyright:** 2002 Eric Espie, Jean-Christophe Durieu; 2005 Olaf Sassnick,
  Bernhard Wymann; 2007 Bernhard Wymann; 2009 Jean-Philippe Meuret;
  2011, 2013, 2024 W.E.C; 2011 Eckhard M. Jäger; 2021 Chad Phillips.

Required credit line:

> Boxer 96 Carson Tuners Edition wheel by Eric Espie, Jean-Christophe Durieu,
> Olaf Sassnick, Bernhard Wymann, Jean-Philippe Meuret, W.E.C, Eckhard M. Jäger,
> and Chad Phillips, Free Art License 1.3, via Speed Dreams.

This attribution covers copyright only. Porsche and 911/996 are trademarks of
their respective owners.

## Files

- `wheel0.acc`: source wheel.
- `wheel3d.png`: wheel texture.

## Modifications by Lapster

- Exported the wheel to glTF with Assimp.
- Reused one wheel mesh for all four corners.
- Scaled it per axle to the Deckard Conejo's source tire dimensions.
- Mirrored the right-side wheel so the rim faces outward.
