import type { WheelFfb } from "../input/wheel-ffb.ts";

const STORAGE_KEY = "lapster.wheel.v1";
const STYLE_ID = "lapster-wheel-controls-style";

interface WheelSettings {
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
`;

function loadSettings(): WheelSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const value = JSON.parse(raw) as Partial<WheelSettings>;
    return {
      strength:
        typeof value.strength === "number" ? Math.min(1, Math.max(0, value.strength)) : 0.5,
      invertForce: value.invertForce === true,
      invertSteering: value.invertSteering === true,
      deadzone:
        typeof value.deadzone === "number" ? Math.min(0.5, Math.max(0, value.deadzone)) : 0.02,
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

  let settings = loadSettings();

  const apply = () => {
    wheel.setStrength(settings.strength);
    wheel.setInvertForce(settings.invertForce);
    wheel.setInvertSteering(settings.invertSteering);
    wheel.setDeadzone(settings.deadzone);
    strength.value = String(settings.strength);
    strengthValue.textContent = `${Math.round(settings.strength * 100)}%`;
    deadzone.value = String(settings.deadzone);
    deadzoneValue.textContent = `${Math.round(settings.deadzone * 100)}%`;
    invertForce.checked = settings.invertForce;
    invertSteering.checked = settings.invertSteering;
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
