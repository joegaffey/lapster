# Audio Specification

## Status

The current Drive audio is a synthesized spike. It uses the vendored MIT-licensed
[Engine_Sim](https://github.com/YigitSalihEmecen/Engine_Sim) runtime as the engine voice and
adds road, surface, and tire layers in the Drive audio graph. No recorded engine or road
samples are bundled.

The original SPEC target remains a Rust/Wasm `sim-audio` crate reading SAB snapshots directly.
The spike deliberately uses a main-thread bridge first so the sound can be tuned before the
DSP is moved behind the worker boundary.

## Runtime topology

- Audio starts **off** on every Drive load. The toolbar `Audio` button creates/resumes the
  `AudioContext` from a user gesture and enables the graph.
- The main thread reads vehicle telemetry each frame and drives Engine_Sim through
  `EngineSim.updateExternal()`.
- Engine_Sim owns its AudioWorklet combustion source and Web Audio exhaust/intake/filter
  graph. Its own drivetrain is bypassed.
- The surface and tire layers are Web Audio noise graphs driven from the same telemetry.
- All layers share the master Drive gain, so mute applies to the complete mix.
- The audio HUD row shows the current wheel surface names and the current road-noise target
  gain.

## Engine voice

- Engine profile: `rotary2`, with external vehicle RPM, speed, gear, throttle, brake, and
  conditioned load.
- External-state adapter:
  - locks the clutch for audio purposes;
  - uses a 60 ms RPM low-pass with a 12,000 rpm/s slew limit outside shifts;
  - shortens the RPM transition to 15 ms during a forward gear change;
  - uses fast load attack and 320 ms release;
  - keeps a 0.38 overrun load floor with reduced overrun attenuation;
  - disables the false transmission lash rattle.
- EngineSim transmission mesh whine is currently muted (`mix.transmission = 0`).
- Forward gear changes trigger a short mechanical click/thump cue. No sampled one-shots.
- Engine braking/overrun in the vehicle model is separate from the audio load envelope.

## Surface and tire layers

Surface selection uses the per-wheel collision surface name, not only friction:

| Surface | Sound |
| --- | --- |
| `ASPHALT` / `ASP*` | Low-pass asphalt roar |
| `ANELLO` | Low-pass dirt rumble plus grit and slow amplitude modulation |
| `KERB` | Low-mid patter |
| `GRASS` | Low rumble plus a low rustle band; reduced tire squeal |

Skidpad's tan `TRANSITION` apron is tagged `ANELLO` by the AC track importer. The `ANELLO`
and grass buses each have two low-frequency amplitude modulators for rough-ground texture.

Tire screech is a separate two-band noise layer. It is gated by skid excess rather than normal
cornering slip:

- lateral skid threshold: 0.09 rad;
- longitudinal skid threshold: 0.14;
- gain scales with excess, tire load, vehicle speed, and surface;
- grass reduces tire squeal.

## Timing and mixing

- Engine_Sim AudioWorklet: 128-frame render quantum.
- Surface and tire updates: main-thread frame cadence, with audio-parameter smoothing.
- No audio samples or convolution reverb yet.
- Surface/tire panning is centered mono.
- Audio does not yet run in the HID/audio worker and does not read SAB snapshots directly.

## Validation

- `web/tests/audio.spec.ts` covers context startup, mute, RPM conditioning, tire screech from
  lateral slip and straight-line wheelspin, and asphalt surface noise.
- The Drive HUD exposes surface and road gain for manual verification.
- Manual checks should cover engine load/overrun, ANELLO/grass/kerb transitions, and audio
  start/mute behavior.

## Credits and licensing

- **Engine_Sim** by Yigit Salih Emecen, MIT:
  https://github.com/YigitSalihEmecen/Engine_Sim
- Vendored source and license: `web/src/audio/engine-sim/`.
- Distribution notices: `THIRD_PARTY_NOTICES.md`.
- No upstream Engine_Sim sample MP3s or images are included.

## Deferred work

- Direct SAB snapshot reads from an audio worklet.
- Per-car audio profiles and rotary/piston metadata in asset packs.
- Optional licensed one-shot or sample layer.
- Stereo/axle panning, surface-specific kerb/gravel detail, Doppler, and reverb zones.
- HID/FFB/audio integration in the worker topology.
