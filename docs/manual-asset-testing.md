# Manual Asset Testing

Real car and track assets are user-supplied and are never committed. Use this checklist
before declaring a real asset supported.

## Placement

1. Create `fixtures/real/cars/<name>/` or `fixtures/real/tracks/<name>/`.
2. Put the source files there.
3. Confirm `fixtures/real/` is ignored by git.
4. Record the source and license in a local note. Do not commit the asset or its textures.

## Compiler checks

- [ ] `asset-compiler inspect` reports the expected asset kind and ID.
- [ ] `asset-compiler validate` accepts the pack.
- [ ] Units and axes are converted to meters, `+Y` up, `-Z` forward.
- [ ] Required chunks exist and hashes match.
- [ ] Chunk offsets satisfy their declared alignment.
- [ ] Unsupported source features appear as warnings, not silent drops.
- [ ] Re-baking the same source produces the same hashes.

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
