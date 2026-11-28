# Audio Specification

## Status

The current Drive audio is sample-based. It plays recorded Speed Dreams engine
loops (with baked fallback WAVs) for the player and opponents, plus road,
surface, and tire layers from the shared `SurfaceAudio` graph. The vendored
MIT-licensed [Engine_Sim](https://github.com/YigitSalihEmecen/Engine_Sim) runtime
is retained as a legacy synth/diagnostic path; the Drive runtime does not
instantiate it.

Opponent audio (local AI and remote multiplayer voices) uses the same sampled
engine profiles. It defaults to three opponent voices, persisted as a 0-5
"Opponent audio" setting under Settings > Audio. The lite oscillator path is
kept only as a diagnostic.

The original SPEC target remains a Rust/Wasm `sim-audio` crate reading SAB snapshots directly.
The spike deliberately uses a main-thread bridge first so the sound can be tuned before the
DSP is moved behind the worker boundary.

## Runtime topology

- Audio is **on by default**. The first pointer/key gesture starts or resumes the graph; the
  toolbar `Audio` button toggles mute or starts it manually. Enabled/muted state is saved as
  `version: 2`; legacy `enabled: false` entries migrate to on. Browsers still require a gesture
  to start the `AudioContext`.
- The main thread reads vehicle telemetry each frame and drives `SampleEngineAudio`; the
  legacy EngineSim path is not started.
- The player sample voice loops the recorded/fallback buffer, with playback rate from
  conditioned RPM and gain from load; a limiter protects the mix.
- The surface and tire layers are Web Audio graphs in `SurfaceAudio`; they share the player's
  `AudioContext`.
- The player mix uses the master gain and an engine-only trim; the surface/tyre layer is
  balanced separately.
- Opponent audio is managed by `SampleRemoteEngineAudio`. All opponent voices share one
  remote `AudioContext`, which is separate from the player context so a failed AI voice cannot
  take the player audio down. The remote class supports a `contextProvider` for a single
  player+AI context, but Drive keeps them separate (mobile isolation, `5fa2622`).
- The audio HUD row shows the current wheel surface names and the current road-noise target
  gain.

## Engine voice

- The player and opponents use recorded Speed Dreams engine loops with baked fallbacks. The
  car/engine profile selects a sample and its `rateMin`/`rateMax`, `refRpm`, gain, load gain,
  and curve from `CAR_MAPPINGS`.
- Playback rate follows RPM, clamped to the mapped RPM window, with short conditioning/slew
  of the telemetry. Gain follows load; an engine-only trim balances the engine against the
  surface/tyre layer.
- The old EngineSim synth (`EngineSimAudio`) remains in the repo as a legacy/diagnostic path;
  the Drive runtime does not instantiate it.
- The old EngineSim transmission and overrun details are legacy and are not part of the
  sample default.

## Opponent audio

- Applies to local AI opponents and, where a room supplies per-player cars, remote
  multiplayer cars. Multiplayer still fixes one car per room until the roster-car batch
  (TD-0025), so current MP voices share that car's profile.
- Default setting: 3 voices. Values 0-5 are stored in
  `lapster.audio.aiVoices.v1`; the Settings > Audio panel exposes "Opponent audio".
  `?aiAudio=off|nearest|full|lite` remains a diagnostic override.
- 1 uses one sample voice for the nearest car, switching engine/vehicle profile as the
  nearest car changes. 2-5 use one sample voice per AI slot, capped by the selected value.
  0 disables opponent audio.
- All opponent voices share one remote AudioContext, separate from the player context. The
  sample remote supports a `contextProvider` for a single player+AI context, but Drive keeps
  them separate for mobile failure isolation (`5fa2622`). Distance attenuation cuts off at
  250 m with a gentler falloff than the original prototype.
- AI frames are interpolated by the Drive render path; rpm, throttle, brake, gear, and wheel
  state come from the 64-float snapshot.
- The lite oscillator mode is diagnostic only and is not the default. It was too loud and too
  synthetic for normal use.

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
