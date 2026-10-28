import type { WheelFfb } from "../input/wheel-ffb.ts";

const STORAGE_KEY = "lapster.wheel.v1";
const STYLE_ID = "lapster-wheel-panel-style";

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
#wheel-dialog {
  width: min(420px, calc(100vw - 24px));
  padding: 0;
  border: 1px solid #2a303a;
  border-radius: 4px;
  background: #0d1117;
  color: #e6e8eb;
  font: 11px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
#wheel-dialog::backdrop { background: rgba(0, 0, 0, 0.58); }
#wheel-dialog .wheel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 11px 14px;
  border-bottom: 1px solid #2a303a;
}
#wheel-dialog h2 { margin: 0; font-size: 13px; }
#wheel-dialog .wheel-body {
  display: grid;
  gap: 8px;
  padding: 12px 14px;
}
#wheel-dialog .wheel-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
#wheel-dialog .wheel-field {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 4px 8px;
  align-items: center;
}
#wheel-dialog .wheel-field input[type="range"] {
  grid-column: 1 / -1;
  width: 100%;
  accent-color: #38bdf8;
}
#wheel-dialog output { color: #8f9bab; }
#wheel-dialog button {
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: #151a21;
  color: #e6e8eb;
  padding: 5px 8px;
  font: inherit;
  cursor: pointer;
}
#wheel-dialog button:hover { background: #1c2430; }
#wheel-dialog button:disabled { opacity: 0.5; cursor: default; }
#wheel-dialog .wheel-status {
  min-height: 32px;
  padding: 7px 9px;
  border: 1px solid #202630;
  background: #080a0d;
  color: #8f9bab;
  white-space: pre-line;
}
#wheel-dialog .wheel-status.ready { color: #34d399; }
#wheel-dialog .wheel-status.error { color: #f87171; }
#wheel-dialog input[type="checkbox"] { accent-color: #38bdf8; }
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

export function setupWheelPanel(wheel: WheelFfb): void {
  if (document.querySelector("#wheel-open")) {
    return;
  }
  if (!document.querySelector(`#${STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.append(style);
  }

  const open = document.createElement("button");
  open.id = "wheel-open";
  open.type = "button";
  open.textContent = "Wheel";
  open.setAttribute("aria-haspopup", "dialog");
  (document.querySelector<HTMLElement>("#viewer-controls") ?? document.body).append(open);

  const dialog = document.createElement("dialog");
  dialog.id = "wheel-dialog";
  dialog.innerHTML = `
    <header class="wheel-head">
      <h2>Wheel / FFB</h2>
      <button id="wheel-close" type="button">Close</button>
    </header>
    <div class="wheel-body">
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
    </div>`;
  document.body.append(dialog);

  const status = dialog.querySelector<HTMLElement>("#wheel-status")!;
  const connectButton = dialog.querySelector<HTMLButtonElement>("#wheel-connect")!;
  const disconnectButton = dialog.querySelector<HTMLButtonElement>("#wheel-disconnect")!;
  const strength = dialog.querySelector<HTMLInputElement>("#wheel-strength")!;
  const strengthValue = dialog.querySelector<HTMLOutputElement>("#wheel-strength-value")!;
  const deadzone = dialog.querySelector<HTMLInputElement>("#wheel-deadzone")!;
  const deadzoneValue = dialog.querySelector<HTMLOutputElement>("#wheel-deadzone-value")!;
  const invertForce = dialog.querySelector<HTMLInputElement>("#wheel-invert-force")!;
  const invertSteering = dialog.querySelector<HTMLInputElement>("#wheel-invert-steering")!;
  const test = dialog.querySelector<HTMLInputElement>("#wheel-test")!;
  const testValue = dialog.querySelector<HTMLOutputElement>("#wheel-test-value")!;
  const testApply = dialog.querySelector<HTMLButtonElement>("#wheel-test-apply")!;
  const stop = dialog.querySelector<HTMLButtonElement>("#wheel-stop")!;
  const close = dialog.querySelector<HTMLButtonElement>("#wheel-close")!;

  let settings = loadSettings();
  wheel.setStrength(settings.strength);
  wheel.setInvertForce(settings.invertForce);
  wheel.setInvertSteering(settings.invertSteering);
  wheel.setDeadzone(settings.deadzone);

  const persist = () => {
    saveSettings(settings);
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
  persist();

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
    persist();
  });
  deadzone.addEventListener("input", () => {
    settings.deadzone = Number(deadzone.value);
    persist();
  });
  invertForce.addEventListener("change", () => {
    settings.invertForce = invertForce.checked;
    persist();
  });
  invertSteering.addEventListener("change", () => {
    settings.invertSteering = invertSteering.checked;
    persist();
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
  close.addEventListener("click", () => dialog.close());
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) dialog.close();
  });
  open.addEventListener("click", () => {
    refresh();
    dialog.showModal();
  });

  const poll = window.setInterval(() => {
    if (dialog.open) refresh();
  }, 250);
  window.addEventListener("pagehide", () => window.clearInterval(poll));
}
