export const COCKPIT_RPM_OFFSETS: Record<string, readonly [number, number, number]> = {
  "67gp-cavallo-123": [0.002, -0.024, -0.01],
  "67gp-kuiper-t81": [0, -0.05, -0.01],
  "67gp-mango-ms7": [0, -0.05, -0.01],
  "67gp-murasama-37a": [0, 0.02, -0.12],
  "67gp-upping-ut19": [0, -0.1, -0.06],
};

export function cockpitRpmOffset(carId: string) {
  return COCKPIT_RPM_OFFSETS[carId];
}
