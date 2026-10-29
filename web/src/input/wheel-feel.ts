const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

export interface WheelFeelSettings {
  rearLightening: number;
  rearSlipThreshold: number;
  frontShudder: number;
  frontSlipThreshold: number;
  shudderHz: number;
  roadNoise: number;
  roughness: number;
  roughnessHz: number;
  looseness: number;
  looseSlipThreshold: number;
}

export interface WheelFeelState {
  speedMps: number;
  frontSlip: number;
  rearSlip: number;
}

export const DEFAULT_FEEL: WheelFeelSettings = {
  rearLightening: 0.7,
  rearSlipThreshold: 0.08,
  frontShudder: 0.8,
  frontSlipThreshold: 0.06,
  shudderHz: 22,
  roadNoise: 0.55,
  roughness: 0.65,
  roughnessHz: 42,
  looseness: 0.45,
  looseSlipThreshold: 0.06,
};

const TAU = Math.PI * 2;

export function computeFfbOutput(
  baseTorque: number,
  state: WheelFeelState,
  settings: WheelFeelSettings,
  timeSeconds: number,
  noise: number,
): number {
  const speed = clamp(state.speedMps / 25, 0, 1);
  const rearExcess = clamp(
    (state.rearSlip - settings.rearSlipThreshold) / 0.35,
    0,
    1,
  );
  const frontExcess = clamp(
    (state.frontSlip - settings.frontSlipThreshold) / 0.3,
    0,
    1,
  );
  const combinedSlip = Math.max(state.frontSlip, state.rearSlip);
  const looseExcess = clamp(
    (combinedSlip - settings.looseSlipThreshold) / 0.5,
    0,
    1,
  );
  const lightening = clamp(
    rearExcess * settings.rearLightening + looseExcess * settings.looseness,
    0,
    0.95,
  );
  const scaledBase = baseTorque * (1 - lightening);
  const shudder =
    Math.sin(TAU * settings.shudderHz * timeSeconds) *
    frontExcess *
    settings.frontShudder *
    0.32;
  const rough =
    (noise * 0.6 + Math.sin(TAU * settings.roughnessHz * timeSeconds) * 0.4) *
    settings.roughness *
    speed *
    (0.05 + 0.15 * combinedSlip);
  const road = noise * settings.roadNoise * speed * 0.12;
  return clamp(scaledBase + shudder + rough + road, -1, 1);
}
