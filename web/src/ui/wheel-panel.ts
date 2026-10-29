import type { WheelFfb } from "../input/wheel-ffb.ts";
import {
  DEFAULT_FEEL,
  type WheelFeelSettings,
} from "../input/wheel-feel.ts";
import { validateProfile } from "../vendor/webhid-wheel/webhid-wheel.js";
import type { WebHIDConfigPanel } from "../vendor/webhid-wheel/src/ui/index.js";
import "../vendor/webhid-wheel/src/ui/index.js";

const PROFILE_STORAGE_KEY = "lapster.wheel.v3";
const FEEL_STORAGE_KEY = "lapster.wheel.feel.v1";
const STORAGE_KEY = "lapster.wheel.v2";
const LEGACY_STORAGE_KEY = "lapster.wheel.v1";
const STYLE_ID = "lapster-wheel-controls-style";

interface LegacyWheelSettings extends WheelFeelSettings {
  strength: number;
  invertForce: boolean;
  invertSteering: boolean;
  deadzone: number;
}

interface FeelControl {
  key: keyof WheelFeelSettings;
  id: string;
  label: string;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
}

const DEFAULTS: LegacyWheelSettings = {
  strength: 0.5,
  invertForce: false,
  invertSteering: false,
  deadzone: 0.02,
  ...DEFAULT_FEEL,
};

const FEEL_CONTROLS: FeelControl[] = [
  {
    key: "rearLightening",
    id: "wheel-rear-lightening",
    label: "Rear breakaway lightening",
    min: 0,
    max: 1,
    step: 0.05,
    format: (value) => `${Math.round(value * 100)}%`,
  },
  {
    key: "rearSlipThreshold",
    id: "wheel-rear-threshold",
    label: "Rear slip threshold",
    min: 0,
    max: 0.8,
    step: 0.01,
    format: (value) => value.toFixed(2),
  },
  {
    key: "frontShudder",
    id: "wheel-front-shudder",
    label: "Front grip shudder",
    min: 0,
    max: 1,
    step: 0.05,
    format: (value) => `${Math.round(value * 100)}%`,
  },
  {
    key: "frontSlipThreshold",
    id: "wheel-front-threshold",
    label: "Front slip threshold",
    min: 0,
    max: 0.8,
    step: 0.01,
    format: (value) => value.toFixed(2),
  },
  {
    key: "shudderHz",
    id: "wheel-shudder-hz",
    label: "Shudder frequency",
    min: 4,
    max: 80,
    step: 1,
    format: (value) => `${Math.round(value)} Hz`,
  },
  {
    key: "roadNoise",
    id: "wheel-road-noise",
    label: "Road noise",
    min: 0,
    max: 1,
    step: 0.05,
    format: (value) => `${Math.round(value * 100)}%`,
  },
  {
    key: "roughness",
    id: "wheel-roughness",
    label: "Roughness",
    min: 0,
    max: 1,
    step: 0.05,
    format: (value) => `${Math.round(value * 100)}%`,
  },
  {
    key: "roughnessHz",
    id: "wheel-roughness-hz",
    label: "Roughness frequency",
    min: 4,
    max: 120,
    step: 1,
    format: (value) => `${Math.round(value)} Hz`,
  },
  {
    key: "looseness",
    id: "wheel-looseness",
    label: "Loose response",
    min: 0,
    max: 1,
    step: 0.05,
    format: (value) => `${Math.round(value * 100)}%`,
  },
  {
    key: "looseSlipThreshold",
    id: "wheel-loose-threshold",
    label: "Loose slip threshold",
    min: 0,
    max: 0.8,
    step: 0.01,
    format: (value) => value.toFixed(2),
  },
];

const STYLE = `
#assists-dialog .wheel-controls {
  display: grid;
  gap: 10px;
}
#assists-dialog .wheel-controls webhid-config-panel {
  display: block;
  --wheel-bg-color: #0d1117;
  --wheel-fg-color: #e6e8eb;
  --wheel-muted-color: #8f9bab;
  --wheel-accent: #38bdf8;
  --wheel-border-color: #2a303a;
  --wheel-radius: 4px;
}
#assists-dialog .lapster-wheel-feel {
  display: grid;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid #202630;
}
#assists-dialog .lapster-wheel-feel h3 {
  margin: 4px 0 0;
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #cbd5e1;
}
#assists-dialog .lapster-wheel-feel .wheel-field {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 4px 8px;
  align-items: center;
}
#assists-dialog .lapster-wheel-feel .wheel-field input[type="range"] {
  grid-column: 1 / -1;
  width: 100%;
  accent-color: #38bdf8;
}
#assists-dialog .lapster-wheel-feel output { color: #8f9bab; }
#assists-dialog .lapster-wheel-feel .wheel-row {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}
`;

const clamp = (value: number, min: number, max: number, fallback: number): number =>
  Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;

function sanitizeFeel(raw: Partial<Record<keyof WheelFeelSettings, unknown>> | null): WheelFeelSettings {
  return {
    rearLightening: clamp(Number(raw?.rearLightening), 0, 1, DEFAULT_FEEL.rearLightening),
    rearSlipThreshold: clamp(Number(raw?.rearSlipThreshold), 0, 0.8, DEFAULT_FEEL.rearSlipThreshold),
    frontShudder: clamp(Number(raw?.frontShudder), 0, 1, DEFAULT_FEEL.frontShudder),
    frontSlipThreshold: clamp(Number(raw?.frontSlipThreshold), 0, 0.8, DEFAULT_FEEL.frontSlipThreshold),
    shudderHz: clamp(Number(raw?.shudderHz), 4, 80, DEFAULT_FEEL.shudderHz),
    roadNoise: clamp(Number(raw?.roadNoise), 0, 1, DEFAULT_FEEL.roadNoise),
    roughness: clamp(Number(raw?.roughness), 0, 1, DEFAULT_FEEL.roughness),
    roughnessHz: clamp(Number(raw?.roughnessHz), 4, 120, DEFAULT_FEEL.roughnessHz),
    looseness: clamp(Number(raw?.looseness), 0, 1, DEFAULT_FEEL.looseness),
    looseSlipThreshold: clamp(Number(raw?.looseSlipThreshold), 0, 0.8, DEFAULT_FEEL.looseSlipThreshold),
  };
}

function readLegacySettings(): LegacyWheelSettings {
  try {
    const raw =
      localStorage.getItem(STORAGE_KEY) ??
      localStorage.getItem(LEGACY_STORAGE_KEY);
    if (!raw) {
      return { ...DEFAULTS };
    }
    const value = JSON.parse(raw) as Partial<LegacyWheelSettings>;
    return {
      strength: clamp(Number(value.strength), 0, 1, DEFAULTS.strength),
      invertForce: value.invertForce === true,
      invertSteering: value.invertSteering === true,
      deadzone: clamp(Number(value.deadzone), 0, 0.5, DEFAULTS.deadzone),
      ...sanitizeFeel(value),
    };
  } catch {
    return { ...DEFAULTS };
  }
}

function loadFeel(legacy: WheelFeelSettings): WheelFeelSettings {
  try {
    const raw = localStorage.getItem(FEEL_STORAGE_KEY);
    if (!raw) {
      return sanitizeFeel(legacy);
    }
    return sanitizeFeel(JSON.parse(raw));
  } catch {
    return sanitizeFeel(legacy);
  }
}

function saveFeel(feel: WheelFeelSettings): void {
  try {
    localStorage.setItem(FEEL_STORAGE_KEY, JSON.stringify(feel));
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
}

function feelMarkup(): string {
  return FEEL_CONTROLS.map(
    (control) => `
      <label class="wheel-field">
        <span>${control.label}</span>
        <output id="${control.id}-value"></output>
        <input id="${control.id}" type="range" min="${control.min}" max="${control.max}" step="${control.step}" />
      </label>`,
  ).join("");
}

function hasLegacySettings(): boolean {
  try {
    return (
      localStorage.getItem(STORAGE_KEY) !== null ||
      localStorage.getItem(LEGACY_STORAGE_KEY) !== null
    );
  } catch {
    return false;
  }
}

function hasProfileStore(): boolean {
  try {
    return localStorage.getItem(PROFILE_STORAGE_KEY) !== null;
  } catch {
    return false;
  }
}

function profileStore(profile: ReturnType<typeof validateProfile>) {
  return {
    kind: "webhid-wheel-profile-store" as const,
    schemaVersion: 2,
    activeProfile: profile.profileName,
    profiles: [profile],
  };
}

export function setupWheelControls(wheel: WheelFfb, host: HTMLElement): void {
  if (!document.querySelector(`#${STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.append(style);
  }

  const legacy = readLegacySettings();
  let feel = loadFeel(legacy);

  host.classList.add("wheel-controls");
  host.innerHTML = `
    <webhid-config-panel tabbed storage-key="${PROFILE_STORAGE_KEY}">
      <div slot="extra-settings" class="lapster-wheel-feel">
        <h3>Feel</h3>
        ${feelMarkup()}
        <div class="wheel-row">
          <button id="wheel-feel-reset" type="button">Reset feel</button>
        </div>
      </div>
    </webhid-config-panel>`;

  const panel = host.querySelector<WebHIDConfigPanel>("webhid-config-panel")!;
  panel.manager = wheel.manager;

  const controls = FEEL_CONTROLS.map((control) => {
    const input = host.querySelector<HTMLInputElement>(`#${control.id}`)!;
    const output = host.querySelector<HTMLOutputElement>(`#${control.id}-value`)!;
    return { control, input, output };
  });
  const resetFeel = host.querySelector<HTMLButtonElement>("#wheel-feel-reset")!;

  const renderFeel = () => {
    for (const { control, input, output } of controls) {
      const value = feel[control.key];
      input.value = String(value);
      output.textContent = control.format(value);
    }
  };

  const applyFeel = (next: WheelFeelSettings, persist = true) => {
    feel = sanitizeFeel(next);
    wheel.setFeel(feel);
    renderFeel();
    if (persist) {
      saveFeel(feel);
    }
  };

  renderFeel();
  applyFeel(feel, false);

  if (!hasProfileStore() && hasLegacySettings()) {
    wheel.setStrength(legacy.strength);
    wheel.setInvertForce(legacy.invertForce);
    wheel.setInvertSteering(legacy.invertSteering);
    wheel.setDeadzone(legacy.deadzone);
  }

  for (const { control, input } of controls) {
    input.addEventListener("input", () => {
      applyFeel({
        ...feel,
        [control.key]: Number(input.value),
      });
    });
  }
  resetFeel.addEventListener("click", () => {
    applyFeel({ ...DEFAULT_FEEL });
  });

  const originalToSetup = wheel.manager.toSetup.bind(wheel.manager);
  wheel.manager.toSetup = (options = {}) => {
    const metadata = options.metadata ?? {};
    return originalToSetup({
      ...options,
      metadata: {
        ...metadata,
        lapster: {
          ...(metadata.lapster ?? {}),
          ...feel,
        },
      },
    });
  };

  const migrateLegacyProfile = () => {
    try {
      if (hasProfileStore()) {
        return;
      }
      const raw =
        localStorage.getItem(STORAGE_KEY) ??
        localStorage.getItem(LEGACY_STORAGE_KEY);
      if (!raw) {
        return;
      }
      const entry = wheel.manager.sessionList.find(
        (item) =>
          item.session?.isConnected &&
          item.capabilities.steering &&
          item.device,
      );
      if (!entry || !panel.profiles) {
        return;
      }
      const profile = validateProfile(
        {
          ...legacy,
          profileName: `${entry.name || "Wheel"} setup`,
        },
        { hardware: entry.hardware },
      );
      panel.profiles.saveStore(profileStore(profile));
      panel.storageKey = PROFILE_STORAGE_KEY;
    } catch (error) {
      console.warn("lapster wheel profile migration failed", error);
    }
  };

  wheel.manager.on("connect", migrateLegacyProfile);

  wheel.manager.on("setup-applied", (payload) => {
    const metadata = payload?.profile?.metadata;
    const lapster = metadata && typeof metadata === "object" ? metadata.lapster : null;
    if (!lapster || typeof lapster !== "object") {
      return;
    }
    applyFeel({ ...feel, ...lapster });
  });

  migrateLegacyProfile();
}
