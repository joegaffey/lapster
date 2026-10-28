import type { WheelFfb } from "../input/wheel-ffb.ts";
import {
  DEFAULT_FEEL,
  type WheelFeelSettings,
} from "../input/wheel-feel.ts";

const STORAGE_KEY = "lapster.wheel.v1";
const STYLE_ID = "lapster-wheel-controls-style";

interface WheelSettings extends WheelFeelSettings {
  strength: number;
  invertForce: boolean;
  invertSteering: boolean;
  deadzone: number;
}

const DEFAULTS: WheelSettings = {
  strength: 0.5,
  invertForce: false,
  invertSteering: false,
  deadzone: 0.02,
  ...DEFAULT_FEEL,
};

const STYLE = `
#assists-dialog .wheel-controls {
  display: grid;
  gap: 8px;
}
#assists-dialog .wheel-controls .wheel-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
#assists-dialog .wheel-controls .wheel-field {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 4px 8px;
  align-items: center;
}
#assists-dialog .wheel-controls .wheel-field input[type="range"] {
  grid-column: 1 / -1;
  width: 100%;
  accent-color: #38bdf8;
}
#assists-dialog .wheel-controls output { color: #8f9bab; }
#assists-dialog .wheel-controls .wheel-status {
  min-height: 32px;
  padding: 7px 9px;
  border: 1px solid #202630;
  background: #080a0d;
  color: #8f9bab;
  white-space: pre-line;
}
#assists-dialog .wheel-controls .wheel-status.ready { color: #34d399; }
#assists-dialog .wheel-controls .wheel-status.error { color: #f87171; }
#assists-dialog .wheel-controls input[type="checkbox"] { accent-color: #38bdf8; }
#assists-dialog .wheel-controls details {
  border-top: 1px solid #202630;
  padding-top: 7px;
}
#assists-dialog .wheel-controls details > summary {
  padding: 3px 0 7px;
  color: #cbd5e1;
  cursor: pointer;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  user-select: none;
}
#assists-dialog .wheel-controls .wheel-feel-grid {
  display: grid;
  gap: 6px;
}
`;

function loadSettings(): WheelSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const value = JSON.parse(raw) as Partial<WheelSettings>;
    const number = (next: unknown, fallback: number, min: number, max: number) =>
      typeof next === "number" ? Math.min(max, Math.max(min, next)) : fallback;
    return {
      strength: number(value.strength, 0.5, 0, 1),
      invertForce: value.invertForce === true,
      invertSteering: value.invertSteering === true,
      deadzone: number(value.deadzone, 0.02, 0, 0.5),
      rearLightening: number(value.rearLightening, DEFAULT_FEEL.rearLightening, 0, 1),
      rearSlipThreshold: number(
        value.rearSlipThreshold,
        DEFAULT_FEEL.rearSlipThreshold,
        0,
        0.8,
      ),
      frontShudder: number(value.frontShudder, DEFAULT_FEEL.frontShudder, 0, 1),
      frontSlipThreshold: number(
        value.frontSlipThreshold,
        DEFAULT_FEEL.frontSlipThreshold,
        0,
        0.8,
      ),
      shudderHz: number(value.shudderHz, DEFAULT_FEEL.shudderHz, 4, 80),
      roadNoise: number(value.roadNoise, DEFAULT_FEEL.roadNoise, 0, 1),
      roughness: number(value.roughness, DEFAULT_FEEL.roughness, 0, 1),
      roughnessHz: number(value.roughnessHz, DEFAULT_FEEL.roughnessHz, 4, 120),
      looseness: number(value.looseness, DEFAULT_FEEL.looseness, 0, 1),
      looseSlipThreshold: number(
        value.looseSlipThreshold,
        DEFAULT_FEEL.looseSlipThreshold,
        0,
        0.8,
      ),
    };
  } catch {
    return { ...DEFAULTS };
  }
}

function saveSettings(settings: WheelSettings): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
}

export function setupWheelControls(wheel: WheelFfb, host: HTMLElement): void {
  if (!document.querySelector(`#${STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.append(style);
  }

  host.classList.add("wheel-controls");
  host.innerHTML = `
    <div id="wheel-status" class="wheel-status"></div>
    <div class="wheel-row">
      <button id="wheel-connect" type="button">Connect wheel</button>
      <button id="wheel-disconnect" type="button" disabled>Disconnect</button>
    </div>
    <label class="wheel-field">
      <span>Force strength</span>
      <output id="wheel-strength-value">50%</output>
      <input id="wheel-strength" type="range" min="0" max="1" step="0.05" />
    </label>
    <label class="wheel-field">
      <span>Steering deadzone</span>
      <output id="wheel-deadzone-value">2%</output>
      <input id="wheel-deadzone" type="range" min="0" max="0.2" step="0.01" />
    </label>
    <label class="wheel-row">
      <span>Invert force</span>
      <input id="wheel-invert-force" type="checkbox" />
    </label>
    <label class="wheel-row">
      <span>Invert steering</span>
      <input id="wheel-invert-steering" type="checkbox" />
    </label>
    <details class="wheel-feel" open>
      <summary>Feel</summary>
      <div class="wheel-feel-grid">
        <label class="wheel-field">
          <span>Rear breakaway lightening</span>
          <output id="wheel-rear-lightening-value">60%</output>
          <input id="wheel-rear-lightening" type="range" min="0" max="1" step="0.05" />
        </label>
        <label class="wheel-field">
          <span>Rear slip threshold</span>
          <output id="wheel-rear-threshold-value">0.12</output>
          <input id="wheel-rear-threshold" type="range" min="0" max="0.8" step="0.01" />
        </label>
        <label class="wheel-field">
          <span>Front grip shudder</span>
          <output id="wheel-front-shudder-value">40%</output>
          <input id="wheel-front-shudder" type="range" min="0" max="1" step="0.05" />
        </label>
        <label class="wheel-field">
          <span>Front slip threshold</span>
          <output id="wheel-front-threshold-value">0.10</output>
          <input id="wheel-front-threshold" type="range" min="0" max="0.8" step="0.01" />
        </label>
        <label class="wheel-field">
          <span>Shudder frequency</span>
          <output id="wheel-shudder-hz-value">18 Hz</output>
          <input id="wheel-shudder-hz" type="range" min="4" max="80" step="1" />
        </label>
        <label class="wheel-field">
          <span>Road noise</span>
          <output id="wheel-road-noise-value">15%</output>
          <input id="wheel-road-noise" type="range" min="0" max="1" step="0.05" />
        </label>
        <label class="wheel-field">
          <span>Roughness</span>
          <output id="wheel-roughness-value">25%</output>
          <input id="wheel-roughness" type="range" min="0" max="1" step="0.05" />
        </label>
        <label class="wheel-field">
          <span>Roughness frequency</span>
          <output id="wheel-roughness-hz-value">32 Hz</output>
          <input id="wheel-roughness-hz" type="range" min="4" max="120" step="1" />
        </label>
        <label class="wheel-field">
          <span>Loose response</span>
          <output id="wheel-looseness-value">35%</output>
          <input id="wheel-looseness" type="range" min="0" max="1" step="0.05" />
        </label>
        <label class="wheel-field">
          <span>Loose slip threshold</span>
          <output id="wheel-loose-threshold-value">0.08</output>
          <input id="wheel-loose-threshold" type="range" min="0" max="0.8" step="0.01" />
        </label>
      </div>
    </details>
    <label class="wheel-field">
      <span>Test force</span>
      <output id="wheel-test-value">0%</output>
      <input id="wheel-test" type="range" min="-1" max="1" step="0.05" value="0" />
    </label>
    <div class="wheel-row">
      <button id="wheel-test-apply" type="button">Apply test force</button>
      <button id="wheel-stop" type="button">Stop FFB</button>
    </div>
    <div class="wheel-row">
      <span></span>
      <button id="wheel-reset" type="button">Reset controls</button>
    </div>`;

  const status = host.querySelector<HTMLElement>("#wheel-status")!;
  const connectButton = host.querySelector<HTMLButtonElement>("#wheel-connect")!;
  const disconnectButton = host.querySelector<HTMLButtonElement>("#wheel-disconnect")!;
  const strength = host.querySelector<HTMLInputElement>("#wheel-strength")!;
  const strengthValue = host.querySelector<HTMLOutputElement>("#wheel-strength-value")!;
  const deadzone = host.querySelector<HTMLInputElement>("#wheel-deadzone")!;
  const deadzoneValue = host.querySelector<HTMLOutputElement>("#wheel-deadzone-value")!;
  const invertForce = host.querySelector<HTMLInputElement>("#wheel-invert-force")!;
  const invertSteering = host.querySelector<HTMLInputElement>("#wheel-invert-steering")!;
  const test = host.querySelector<HTMLInputElement>("#wheel-test")!;
  const testValue = host.querySelector<HTMLOutputElement>("#wheel-test-value")!;
  const testApply = host.querySelector<HTMLButtonElement>("#wheel-test-apply")!;
  const stop = host.querySelector<HTMLButtonElement>("#wheel-stop")!;
  const reset = host.querySelector<HTMLButtonElement>("#wheel-reset")!;
  const rearLightening = host.querySelector<HTMLInputElement>("#wheel-rear-lightening")!;
  const rearLighteningValue = host.querySelector<HTMLOutputElement>(
    "#wheel-rear-lightening-value",
  )!;
  const rearThreshold = host.querySelector<HTMLInputElement>("#wheel-rear-threshold")!;
  const rearThresholdValue = host.querySelector<HTMLOutputElement>(
    "#wheel-rear-threshold-value",
  )!;
  const frontShudder = host.querySelector<HTMLInputElement>("#wheel-front-shudder")!;
  const frontShudderValue = host.querySelector<HTMLOutputElement>(
    "#wheel-front-shudder-value",
  )!;
  const frontThreshold = host.querySelector<HTMLInputElement>("#wheel-front-threshold")!;
  const frontThresholdValue = host.querySelector<HTMLOutputElement>(
    "#wheel-front-threshold-value",
  )!;
  const shudderHz = host.querySelector<HTMLInputElement>("#wheel-shudder-hz")!;
  const shudderHzValue = host.querySelector<HTMLOutputElement>("#wheel-shudder-hz-value")!;
  const roadNoise = host.querySelector<HTMLInputElement>("#wheel-road-noise")!;
  const roadNoiseValue = host.querySelector<HTMLOutputElement>("#wheel-road-noise-value")!;
  const roughness = host.querySelector<HTMLInputElement>("#wheel-roughness")!;
  const roughnessValue = host.querySelector<HTMLOutputElement>("#wheel-roughness-value")!;
  const roughnessHz = host.querySelector<HTMLInputElement>("#wheel-roughness-hz")!;
  const roughnessHzValue = host.querySelector<HTMLOutputElement>(
    "#wheel-roughness-hz-value",
  )!;
  const looseness = host.querySelector<HTMLInputElement>("#wheel-looseness")!;
  const loosenessValue = host.querySelector<HTMLOutputElement>("#wheel-looseness-value")!;
  const looseThreshold = host.querySelector<HTMLInputElement>("#wheel-loose-threshold")!;
  const looseThresholdValue = host.querySelector<HTMLOutputElement>(
    "#wheel-loose-threshold-value",
  )!;

  let settings = loadSettings();

  const apply = () => {
    wheel.setStrength(settings.strength);
    wheel.setInvertForce(settings.invertForce);
    wheel.setInvertSteering(settings.invertSteering);
    wheel.setDeadzone(settings.deadzone);
    wheel.setFeel(settings);
    strength.value = String(settings.strength);
    strengthValue.textContent = `${Math.round(settings.strength * 100)}%`;
    deadzone.value = String(settings.deadzone);
    deadzoneValue.textContent = `${Math.round(settings.deadzone * 100)}%`;
    invertForce.checked = settings.invertForce;
    invertSteering.checked = settings.invertSteering;
    rearLightening.value = String(settings.rearLightening);
    rearLighteningValue.textContent = `${Math.round(settings.rearLightening * 100)}%`;
    rearThreshold.value = String(settings.rearSlipThreshold);
    rearThresholdValue.textContent = settings.rearSlipThreshold.toFixed(2);
    frontShudder.value = String(settings.frontShudder);
    frontShudderValue.textContent = `${Math.round(settings.frontShudder * 100)}%`;
    frontThreshold.value = String(settings.frontSlipThreshold);
    frontThresholdValue.textContent = settings.frontSlipThreshold.toFixed(2);
    shudderHz.value = String(settings.shudderHz);
    shudderHzValue.textContent = `${Math.round(settings.shudderHz)} Hz`;
    roadNoise.value = String(settings.roadNoise);
    roadNoiseValue.textContent = `${Math.round(settings.roadNoise * 100)}%`;
    roughness.value = String(settings.roughness);
    roughnessValue.textContent = `${Math.round(settings.roughness * 100)}%`;
    roughnessHz.value = String(settings.roughnessHz);
    roughnessHzValue.textContent = `${Math.round(settings.roughnessHz)} Hz`;
    looseness.value = String(settings.looseness);
    loosenessValue.textContent = `${Math.round(settings.looseness * 100)}%`;
    looseThreshold.value = String(settings.looseSlipThreshold);
    looseThresholdValue.textContent = settings.looseSlipThreshold.toFixed(2);
  };
  apply();

  const refresh = () => {
    const state = wheel.status();
    const lines = [
      state.available ? "WebHID available" : "WebHID unavailable in this browser",
      state.connected ? `Device: ${state.name || "connected"}` : "Device: not connected",
    ];
    if (state.connected) {
      lines.push(`Parser: ${state.parser || "none"}`);
      lines.push(`Force feedback: ${state.ffb ? "constant force" : "not supported"}`);
      lines.push(`Steering: ${(state.steering * 100).toFixed(0)}%`);
      lines.push(
        `Base ${(state.base * 100).toFixed(0)}% · Output ${(state.output * 100).toFixed(0)}%`,
      );
      lines.push(
        `Slip F ${state.frontSlip.toFixed(2)} / R ${state.rearSlip.toFixed(2)}`,
      );
    }
    if (state.error) {
      lines.push(`Error: ${state.error}`);
    }
    status.textContent = lines.join("\n");
    status.className = "wheel-status";
    if (state.error) status.classList.add("error");
    else if (state.connected) status.classList.add("ready");
    connectButton.disabled = !state.available || state.connected;
    disconnectButton.disabled = !state.connected;
    testApply.disabled = !state.connected || !state.ffb;
    stop.disabled = !state.connected;
  };
  refresh();

  connectButton.addEventListener("click", async () => {
    connectButton.disabled = true;
    try {
      await wheel.connect();
    } catch (error) {
      status.textContent = error instanceof Error ? error.message : String(error);
      status.className = "wheel-status error";
    }
    refresh();
  });
  disconnectButton.addEventListener("click", async () => {
    await wheel.disconnect();
    refresh();
  });
  strength.addEventListener("input", () => {
    settings.strength = Number(strength.value);
    saveSettings(settings);
    apply();
  });
  deadzone.addEventListener("input", () => {
    settings.deadzone = Number(deadzone.value);
    saveSettings(settings);
    apply();
  });
  invertForce.addEventListener("change", () => {
    settings.invertForce = invertForce.checked;
    saveSettings(settings);
    apply();
  });
  invertSteering.addEventListener("change", () => {
    settings.invertSteering = invertSteering.checked;
    saveSettings(settings);
    apply();
  });
  rearLightening.addEventListener("input", () => {
    settings.rearLightening = Number(rearLightening.value);
    saveSettings(settings);
    apply();
  });
  rearThreshold.addEventListener("input", () => {
    settings.rearSlipThreshold = Number(rearThreshold.value);
    saveSettings(settings);
    apply();
  });
  frontShudder.addEventListener("input", () => {
    settings.frontShudder = Number(frontShudder.value);
    saveSettings(settings);
    apply();
  });
  frontThreshold.addEventListener("input", () => {
    settings.frontSlipThreshold = Number(frontThreshold.value);
    saveSettings(settings);
    apply();
  });
  shudderHz.addEventListener("input", () => {
    settings.shudderHz = Number(shudderHz.value);
    saveSettings(settings);
    apply();
  });
  roadNoise.addEventListener("input", () => {
    settings.roadNoise = Number(roadNoise.value);
    saveSettings(settings);
    apply();
  });
  roughness.addEventListener("input", () => {
    settings.roughness = Number(roughness.value);
    saveSettings(settings);
    apply();
  });
  roughnessHz.addEventListener("input", () => {
    settings.roughnessHz = Number(roughnessHz.value);
    saveSettings(settings);
    apply();
  });
  looseness.addEventListener("input", () => {
    settings.looseness = Number(looseness.value);
    saveSettings(settings);
    apply();
  });
  looseThreshold.addEventListener("input", () => {
    settings.looseSlipThreshold = Number(looseThreshold.value);
    saveSettings(settings);
    apply();
  });
  test.addEventListener("input", () => {
    testValue.textContent = `${Math.round(Number(test.value) * 100)}%`;
  });
  testApply.addEventListener("click", () => {
    wheel.testForce(Number(test.value));
    refresh();
  });
  stop.addEventListener("click", () => {
    wheel.stop(true);
    refresh();
  });
  reset.addEventListener("click", () => {
    settings = { ...DEFAULTS };
    saveSettings(settings);
    apply();
    refresh();
  });

  const poll = window.setInterval(() => {
    if (host.isConnected) refresh();
  }, 250);
  window.addEventListener("pagehide", () => window.clearInterval(poll));
}
