# Audio Specification

## Status

The current Drive audio is a synthesized spike. It uses the vendored MIT-licensed
[Engine_Sim](https://github.com/YigitSalihEmecen/Engine_Sim) runtime as the engine voice and
adds road, surface, and tire layers in the Drive audio graph. No recorded engine or road
samples are bundled.

Opponent audio (local AI and remote multiplayer voices) is layered on the same
Engine_Sim graph family. It defaults to one nearest-AI voice on the player's
AudioContext, with a persisted 0-5 "Opponent audio" menu setting. The full
multi-voice path is available for desktops; the lite oscillator path is kept
only as a diagnostic. No recorded opponent engine samples are bundled.

The original SPEC target remains a Rust/Wasm `sim-audio` crate reading SAB snapshots directly.
The spike deliberately uses a main-thread bridge first so the sound can be tuned before the
DSP is moved behind the worker boundary.

## Runtime topology

- Audio starts **off** on first use. The toolbar `Audio` button creates/resumes the
  `AudioContext` from a user gesture and enables the graph. Enabled and muted state are saved
  in `localStorage`; after a reload the button reflects the saved state and audio starts on the
  first user interaction because browsers require a gesture.
- The main thread reads vehicle telemetry each frame and drives Engine_Sim through
  `EngineSim.updateExternal()`.
- Engine_Sim owns its AudioWorklet combustion source and Web Audio exhaust/intake/filter
  graph. Its own drivetrain is bypassed.
- The surface and tire layers are Web Audio noise graphs driven from the same telemetry.
- All layers share the master Drive gain, so mute applies to the complete mix.
- Opponent audio is managed by `RemoteEngineAudio` and owns a separate
  AudioContext from the player engine, so a failed AI voice cannot take the
  player audio down. The nearest-AI default runs one EngineSim voice on a
  dedicated slot; 2-5 voices use one EngineSim voice per AI slot; 0 disables
  opponent audio.
- The audio HUD row shows the current wheel surface names and the current road-noise target
  gain.

## Engine voice

- Engine profile comes from the selected car catalog entry: `rotary2` for the Mazda RX-7
  and `v8cross` (cross-plane V8) for the Speed Dreams Deckard Conejo. Custom packs that are
  not in the catalog use the legacy `rotary2`/`sports` fallback. The profile is driven by
  external vehicle RPM, speed, gear, throttle, brake, and conditioned load.
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

## Opponent audio

- Applies to local AI opponents and, where a room supplies per-player cars, remote
  multiplayer cars. Multiplayer still fixes one car per room until the roster-car batch
  (TD-0025), so current MP voices share that car's profile.
- Default setting: 1 voice. Values 0-5 are stored in
  `lapster.audio.aiVoices.v1`; the app menu exposes "Opponent audio".
  `?aiAudio=off|nearest|full|lite` remains a diagnostic override.
- 1 uses one EngineSim voice for the nearest car, switching engine/vehicle
  profile as the nearest car changes. 2-5 use one EngineSim voice per AI slot,
  capped by the selected value. 0 disables opponent audio.
- Opponent voices use their own AudioContext so the player context stays
  isolated. Distance attenuation cuts off at 250 m with a gentler falloff than
  the original prototype.
- AI frames are interpolated by the Drive render path; rpm, throttle, brake,
  gear, and wheel state come from the 64-float snapshot.
- The lite oscillator mode is diagnostic only and is not the default. It was
  too loud and too synthetic for normal use.

## Surface and tire layers

Surface selection uses the per-wheel collision surface name, not only friction:

| Surface | Sound |
| --- | --- |
| `ASPHALT` / `ASP*` | Low-pass asphalt roar, including Skidpad's banked outer ring |
| `ANELLO` | Concrete apron rumble plus grit and slow amplitude modulation |
| `KERB` | Low-mid patter |
| `GRASS` | Low rumble plus a low rustle band; reduced tire squeal |

Skidpad's tan `TRANSITION` concrete apron is tagged `ANELLO` by the AC track importer. The
`ANELLO` and grass buses each have two low-frequency amplitude modulators for rough-ground
texture.

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
- `web/tests/ai-audio.spec.ts` covers the default nearest voice, the 0-5
  opponent setting, full mode, off mode, per-car profiles, and reload
  persistence. `web/tests/ai-audio-setting.test.ts` covers clamping and
  persistence.
- The Drive HUD exposes surface and road gain for manual verification.
- Manual checks should cover engine load/overrun, banked-asphalt/asphalt/concrete/grass/kerb
  transitions, and audio start/mute behavior.

## Credits and licensing

- **Engine_Sim** by Yigit Salih Emecen, MIT:
  https://github.com/YigitSalihEmecen/Engine_Sim
- Vendored source and license: `web/src/audio/engine-sim/`.
- Distribution notices: `THIRD_PARTY_NOTICES.md`.
- No upstream Engine_Sim sample MP3s or images are included.

## Deferred work

- Direct SAB snapshot reads from an audio worklet.
- Move engine profile, exhaust, and cockpit audio metadata from the web catalog into asset
  packs; custom Speed Dreams cars currently fall back to `rotary2`.
- Sampled opponent engine loops. No per-engine sample assets are in the
  repository, the pack `Audio` chunk is unused, and a sample path needs
  licensing/attribution, loop points, and RPM/load crossfades. This is a future
  asset + audio batch and would likely be cheaper than per-car synthesis for AI.
- Per-player multiplayer opponent audio once TD-0025 carries each player's car
  in the roster; today a room carries one car.
- Stereo/axle panning, surface-specific kerb/gravel detail, Doppler, and reverb zones.
- HID/FFB/audio integration in the worker topology.
