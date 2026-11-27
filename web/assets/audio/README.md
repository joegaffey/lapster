# Engine sample audio (test branch)

This directory is a test-only sample set for `feat/sample-engine-audio`. It is
not intended for merge without a licensing review.

## Recorded samples

Engine-profile loops from the Speed Dreams data repository
(https://forge.a-lec.org/speed-dreams/speed-dreams-data, `data/data/sound/`,
GPL-2.0; see `COPYING-GPL-2.0.txt`):

- `speed-dreams/rotary2.wav` — Mazda RX-7 / rotary2
- `speed-dreams/v8-ears68.wav` — Deckard Conejo / v8cross (exact source)
- `speed-dreams/v12_smooth.wav` — generic V12 stand-in

Per-car source loops from the Speed Dreams 67GP car repositories (Free Art
License 1.3; see `COPYING-FAL-1.3.txt`):

| Car | Source file | Repo |
|---|---|---|
| 67gp-cavallo-123 | `cars/f123bis.wav` | https://forge.a-lec.org/speed-dreams/67gp-cavallo-123 |
| 67gp-guthrie-eagle | `cars/GT3O.wav` | https://forge.a-lec.org/speed-dreams/67gp-guthrie-eagle |
| 67gp-kuiper-t81 | `cars/GT3O.wav` | https://forge.a-lec.org/speed-dreams/67gp-kuiper-t81 |
| 67gp-murasama-37a | `cars/037a.wav` | https://forge.a-lec.org/speed-dreams/67gp-murasama-37a |
| 67gp-mango-ms7 | `cars/formulaford.wav` | https://forge.a-lec.org/speed-dreams/67gp-mango-ms7 |
| 67gp-upping-ut19 | `cars/GT3O.wav` | https://forge.a-lec.org/speed-dreams/67gp-upping-ut19 |

The 67GP car credits are in the corresponding
`fixtures/sources/cars/*/README.md` files. This is a test-only branch; confirm
licensing before merging.

## Baked fallback (`fallback/`)

Generated offline from the vendored Engine_Sim (MIT) using the study bake tool
on `spike/audio-study`, at 4800 rpm and 0.9 load. Used only if a recorded sample
fails to load.

## Test URL knobs

- `?sampleSource=fallback` forces the baked fallback.
- `?sampleEngine=v8cross` forces one sample for every voice.
- `?sampleRefRpm=3500` overrides the sample reference RPM.
- `?sampleRateMin=0.7&sampleRateMax=1.5` overrides the playback-rate range.
- `?sampleGain=0.3` overrides the master gain.
- `?sampleEngineGain=0.55` trims the engine against the noise bus.
- `?sampleNoiseGain=1.6` trims tyre/road noise.
