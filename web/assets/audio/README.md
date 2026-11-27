# Engine sample audio (test branch)

This directory is a test-only sample set for `feat/sample-engine-audio`. It is
not intended for merge without a licensing review.

## Recorded samples (`speed-dreams/`)

- `rotary2.wav`, `v8-ears68.wav`, `v12_smooth.wav` come from the Speed Dreams
  data repository: https://forge.a-lec.org/speed-dreams/speed-dreams-data at
  `data/data/sound/`. That repository is GPL-2.0; the full text is in
  `COPYING-GPL-2.0.txt`.
- Mapping used by the test:
  - Mazda RX-7 / `rotary2` profile -> `rotary2.wav`
  - Deckard Conejo / `v8cross` profile -> `v8-ears68.wav`
  - GP67 V12 profiles -> `v12_smooth.wav` (generic V12 stand-in)

## Baked fallback (`fallback/`)

Generated offline from the vendored Engine_Sim (MIT) using the study bake tool
on `spike/audio-study`, at 4800 rpm and 0.9 load. Used only if a recorded sample
fails to load.

## Test URL knobs

- `?sampleSource=fallback` forces the baked fallback.
- `?sampleRefRpm=3500` overrides the sample reference RPM.
- `?sampleGain=0.3` overrides the master gain.
