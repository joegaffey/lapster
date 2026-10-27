# Manual Asset Testing

User-supplied car and track assets are normally not committed. The checked-in RX-7 and
Skidpad 0.5 packs are temporary demo fixtures; see `fixtures/README.md` and their per-folder
notes. Use this checklist before declaring another real asset supported.

## Placement

1. Create `fixtures/real/cars/<name>/` or `fixtures/real/tracks/<name>/`.
2. Put the source files there.
3. Confirm `fixtures/real/` is ignored by git.
4. Record the source and license in a local note. Do not commit new user assets or their
   textures unless they are explicitly adopted as demo fixtures with attribution.

## Compiler checks

- [ ] `asset-compiler inspect` reports the expected asset kind and ID.
- [ ] `asset-compiler validate` accepts the pack.
- [ ] Units and axes are converted to meters, `+Y` up, `-Z` forward.
- [ ] Required chunks exist and hashes match.
- [ ] Chunk offsets satisfy their declared alignment.
- [ ] Unsupported source features appear as warnings, not silent drops.
- [ ] Re-baking the same source produces the same hashes.

## GLB cars

For a car that already has a `.glb`:

```bash
cargo run -p asset-compiler -- bake-glb fixtures/real/cars/<name>/<car>.glb
cargo run -p asset-compiler -- inspect fixtures/real/cars/<name>/<car>.sima
cargo run -p asset-compiler -- validate fixtures/real/cars/<name>/<car>.sima
```

Open the garage with the local pack:

```text
http://127.0.0.1:8123/web/garage/?car=../../fixtures/real/cars/<name>/<car>.sima
```

The same `car` query parameter works on `/web/drive/`. A `track` query parameter is also
supported. The first GLB pass bakes materials into vertex colours, derives wheel transforms
from node names, removes logo nodes, and uses generic vehicle physics; decimation and
per-vehicle physics mapping are follow-up work.

## AC tracks

For an Assetto Corsa track folder containing `*.kn5` and `data/surfaces.ini`:

```bash
cargo run -p asset-compiler -- bake-ac-track fixtures/real/tracks/<name> \
  fixtures/tracks/<name>/pack.sima
cargo run -p asset-compiler -- inspect fixtures/tracks/<name>/pack.sima
cargo run -p asset-compiler -- validate fixtures/tracks/<name>/pack.sima
```

Open it in the track viewer, garage, or drive viewer:

```text
http://127.0.0.1:8123/web/track-viewer/?track=../../fixtures/real/tracks/<name>/pack.sima
http://127.0.0.1:8123/web/garage/?track=../../fixtures/real/tracks/<name>/pack.sima&car=../../fixtures/real/cars/<car>/<car>.sima
```

The first AC pass parses the KN5 node tree, vehicle/grid markers, `surfaces.ini`, and
drivable geometry. Textures are not imported yet; material colours are approximated. The
outer ANELLO ring is visual-only, trees/fences are skipped, and walls are not in collision.

## Car checks

- [ ] The visual viewer shows the correct scale and orientation.
- [ ] All four wheels are present at the expected positions.
- [ ] Steering wheel and body transforms are plausible.
- [ ] Materials and textures display without obvious color-space errors.
- [ ] The car drives on the synthetic track without manual edits.
- [ ] Key physics values match the source within the documented tolerance.
- [ ] Missing or approximated parameters are listed in the validation report.
- [ ] The car settles on its suspension without NaN or jitter.

## Track checks

- [ ] The visual viewer shows the correct scale and orientation.
- [ ] The collision debug view aligns with the road and walls.
- [ ] The car spawns at the start position and does not fall through the ground.
- [ ] Surface friction IDs map to the expected materials.
- [ ] Pit boxes, sectors, and cameras load.
- [ ] A manual lap completes without collision gaps or invisible walls.
- [ ] Load time and memory are recorded.

## Performance and stability

- [ ] Load time is within the documented budget or a waiver is recorded.
- [ ] Physics step p99 stays within budget for a 60-second drive.
- [ ] No unbounded catch-up events.
- [ ] FFB watchdog sends stop when updates become stale.
- [ ] Reloading from cache works.
- [ ] Corrupt or unsupported assets fail with an actionable error.

## Sign-off

- Asset name:
- Source/license:
- Compiler version:
- Pack hash:
- Tester/date:
- Known issues:
