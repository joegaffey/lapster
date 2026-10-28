import { BUILD_CHANNEL } from "../version.generated.ts";
import { setFpsCounterVisible } from "./fps.ts";

export interface SettingsViewer {
  set_traction_control?(enabled: boolean): void;
  set_traction_strength?(strength: number): void;
  set_abs_control?(enabled: boolean): void;
  set_stability_control?(enabled: boolean): void;
  set_stability_strength?(strength: number): void;
  set_steering_sensitivity?(sensitivity: number): void;
  set_debug_lines_visible(visible: boolean): void;
  set_environment_shadow_mode(mode: string): void;
  set_car_shadow_mode(mode: string): void;
  set_time_of_day(hours: number): number;
  set_msaa_samples(requested: number): number;
  msaa_samples(): number;
}

export type AntiAliasing = "auto" | "off" | "2x" | "4x";
export type EnvironmentShadows = "high" | "hybrid" | "baked" | "off";
export type CarShadow = "dynamic" | "silhouette" | "off";
export type RenderScale = 1 | 0.85 | 0.75 | 0.67 | 0.5;

const RENDER_SCALES: RenderScale[] = [1, 0.85, 0.75, 0.67, 0.5];

export interface AssistSettings {
  tc: boolean;
  tcStrength: number;
  abs: boolean;
  esc: boolean;
  escStrength: number;
  steeringSensitivity: number;
  debugLines: boolean;
  environmentShadows: EnvironmentShadows;
  carShadow: CarShadow;
  timeOfDay: number;
  antiAliasing: AntiAliasing;
  fpsCounter: boolean;
  renderScale: RenderScale;
}

interface SettingsOptions {
  gameplay?: boolean;
  onRenderScale?: (scale: number) => void;
}

export interface SettingsController {
  get(): AssistSettings;
  update(patch: Partial<AssistSettings>): void;
  refresh(): void;
  reset(): void;
}

const DEV_BUILD = BUILD_CHANNEL === "dev";
const STORAGE_KEY = DEV_BUILD ? "lapster.settings.dev.v3" : "lapster.settings.v3";
const PREVIOUS_STORAGE_KEY = DEV_BUILD ? "lapster.settings.dev.v2" : "lapster.settings.v2";
const LEGACY_STORAGE_KEY = DEV_BUILD ? "" : "lapster.assists.v1";
const SETTINGS_STYLE_ID = "lapster-settings-style";

const DEFAULTS: AssistSettings = {
  tc: true,
  tcStrength: 0.5,
  abs: true,
  esc: true,
  escStrength: 0.5,
  steeringSensitivity: 1,
  debugLines: false,
  environmentShadows: "hybrid",
  carShadow: "dynamic",
  timeOfDay: 15,
  antiAliasing: "auto",
  fpsCounter: false,
  renderScale: 1,
};

const SETTINGS_STYLE = `
#assists-dialog {
  width: min(560px, calc(100vw - 24px));
  max-height: min(90vh, 760px);
  padding: 0;
  border: 1px solid #2a303a;
  border-radius: 4px;
  background: #0d1117;
  color: #e6e8eb;
  font: inherit;
  overflow: auto;
}
#assists-dialog::backdrop { background: rgba(0, 0, 0, 0.62); }
#assists-dialog .setup-header {
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid #2a303a;
  background: #0d1117;
}
#assists-dialog .setup-header h2 { margin: 0; font-size: 14px; }
#assists-dialog button,
#assists-dialog select {
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: #151a21;
  color: #e6e8eb;
  padding: 5px 8px;
  font: inherit;
  font-size: 11px;
}
#assists-dialog button { cursor: pointer; }
#assists-dialog button:hover { background: #1c2430; }
#assists-dialog .settings-tabs {
  display: flex;
  gap: 4px;
  padding: 8px 16px 0;
  border-bottom: 1px solid #202630;
}
#assists-dialog .settings-tab {
  border-bottom: 0;
  border-radius: 3px 3px 0 0;
  background: transparent;
  color: #8f9bab;
}
#assists-dialog .settings-tab[aria-selected="true"] {
  background: #151a21;
  color: #e6e8eb;
  border-color: #3a4350;
}
#assists-dialog .settings-panel[hidden] { display: none; }
#assists-dialog .assist-body { padding: 10px 16px 14px; }
#assists-dialog .assist-row {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: 12px;
  padding: 6px 0;
  font-size: 11px;
}
#assists-dialog .assist-slider {
  display: grid;
  grid-template-columns: 1fr auto auto;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  font-size: 11px;
}
#assists-dialog .assist-slider input[type="range"] { grid-column: 1 / -1; width: 100%; accent-color: #38bdf8; }
#assists-dialog .assist-slider output { color: #8f9bab; }
#assists-dialog .assist-slider select { grid-column: 1 / -1; width: 100%; }
#assists-dialog .time-start { color: #8f9bab; }
#assists-dialog .setup-footer {
  position: sticky;
  bottom: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid #2a303a;
  background: #0d1117;
}
#assists-dialog .setup-status { margin: 0; color: #8f9bab; font-size: 11px; }
#assists-dialog .setup-actions { display: flex; gap: 6px; }
@media (max-width: 720px) {
  #assists-dialog .assist-slider { grid-template-columns: 1fr auto; }
  #assists-dialog .assist-slider select { grid-column: 1 / -1; }
}
`;

function settingsDialogHtml(): string {
  return `
<dialog id="assists-dialog" aria-labelledby="assists-title">
  <header class="setup-header">
    <h2 id="assists-title">Settings</h2>
    <button id="assists-close" type="button">Close</button>
  </header>
  <div class="settings-tabs" role="tablist" aria-label="Settings sections">
    <button id="tab-gameplay" class="settings-tab" role="tab" aria-selected="true" aria-controls="settings-gameplay" data-tab="gameplay" type="button">Gameplay</button>
    <button id="tab-graphics" class="settings-tab" role="tab" aria-selected="false" aria-controls="settings-graphics" data-tab="graphics" type="button">Graphics</button>
  </div>
  <section id="settings-gameplay" class="settings-panel" role="tabpanel" aria-labelledby="tab-gameplay">
    <div class="assist-body">
      <div class="assist-row">
        <span>Traction control</span>
        <button id="tc-toggle" data-testid="tc-toggle" aria-pressed="true" type="button">TC: On</button>
      </div>
      <label class="assist-slider">
        <span>TC strength</span>
        <output id="tc-strength-value">50%</output>
        <input id="tc-strength" type="range" min="0" max="1" step="0.05" value="0.5" />
      </label>
      <div class="assist-row">
        <span>Anti-lock brakes</span>
        <button id="abs-toggle" data-testid="abs-toggle" aria-pressed="true" type="button">ABS: On</button>
      </div>
      <div class="assist-row">
        <span>Stability control</span>
        <button id="esc-toggle" data-testid="esc-toggle" aria-pressed="true" type="button">ESC: On</button>
      </div>
      <label class="assist-slider">
        <span>ESC strength</span>
        <output id="esc-strength-value">50%</output>
        <input id="esc-strength" type="range" min="0" max="1" step="0.05" value="0.5" />
      </label>
      <label class="assist-slider">
        <span>Steering sensitivity</span>
        <output id="steering-sensitivity-value">100%</output>
        <input id="steering-sensitivity" type="range" min="0.25" max="2" step="0.05" value="1" />
      </label>
    </div>
  </section>
  <section id="settings-graphics" class="settings-panel" role="tabpanel" aria-labelledby="tab-graphics" hidden>
    <div class="assist-body">
      <label class="assist-slider">
        <span>Anti-aliasing</span>
        <output id="msaa-applied">—</output>
        <select id="msaa-mode">
          <option value="auto">Auto (4× desktop / 2× mobile)</option>
          <option value="off">Off</option>
          <option value="2x">2× MSAA</option>
          <option value="4x">4× MSAA</option>
        </select>
      </label>
      <label class="assist-slider">
        <span>Render scale</span>
        <output id="render-scale-value">100%</output>
        <select id="render-scale">
          <option value="1">100%</option>
          <option value="0.85">85%</option>
          <option value="0.75">75%</option>
          <option value="0.67">67%</option>
          <option value="0.5">50%</option>
        </select>
      </label>
      <label class="assist-slider">
        <span>Environment shadows</span>
        <select id="environment-shadows">
          <option value="baked">Static</option>
          <option value="hybrid">Static + near dynamic</option>
          <option value="high">Dynamic</option>
          <option value="off">Off</option>
        </select>
      </label>
      <label class="assist-slider">
        <span>Car shadow</span>
        <select id="car-shadow">
          <option value="dynamic">Dynamic</option>
          <option value="silhouette">Silhouette</option>
          <option value="off">Off</option>
        </select>
      </label>
      <label class="assist-slider">
        <span>Time of day</span>
        <output id="time-of-day-value">15:00</output>
        <span class="time-start"></span>
        <input id="time-of-day" type="range" min="6" max="20" step="0.5" value="15" />
      </label>
      <div class="assist-row">
        <span>Apply lighting</span>
        <button id="time-of-day-apply" type="button">Apply</button>
      </div>
      <div class="assist-row">
        <span>Debug overlay</span>
        <button id="debug-lines-toggle" aria-pressed="false" type="button">Debug: Off</button>
      </div>
      <div class="assist-row">
        <span>FPS counter</span>
        <button id="fps-counter-toggle" aria-pressed="false" type="button">FPS: Off</button>
      </div>
    </div>
  </section>
  <footer class="setup-footer">
    <p id="assists-status" class="setup-status" aria-live="polite">Changes apply live</p>
    <div class="setup-actions">
      <button id="assists-reset" type="button">Reset settings</button>
    </div>
  </footer>
</dialog>`;
}

function ensureSettingsUi(gameplay: boolean): void {
  if (!document.querySelector(`#${SETTINGS_STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = SETTINGS_STYLE_ID;
    style.textContent = SETTINGS_STYLE;
    document.head.append(style);
  }
  if (!document.querySelector("#assists-dialog")) {
    document.body.insertAdjacentHTML("beforeend", settingsDialogHtml());
  }
  let openButton = document.querySelector<HTMLButtonElement>("#assists-open");
  if (!openButton) {
    openButton = document.createElement("button");
    openButton.id = "assists-open";
    openButton.type = "button";
    openButton.textContent = "Settings";
    openButton.setAttribute("aria-haspopup", "dialog");
    const host = document.querySelector<HTMLElement>("#viewer-controls") ?? document.body;
    host.append(openButton);
  }
  if (!gameplay) {
    document.querySelector("#tab-gameplay")?.remove();
    document.querySelector("#settings-gameplay")?.remove();
    const graphicsTab = document.querySelector<HTMLButtonElement>("#tab-graphics");
    graphicsTab?.setAttribute("aria-selected", "true");
    document.querySelector("#settings-graphics")?.removeAttribute("hidden");
  }
}

function clamp(value: number, min: number, max: number, fallback: number): number {
  return Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;
}

function sanitize(value: Partial<AssistSettings> | null): AssistSettings {
  const environmentShadows =
    value?.environmentShadows === "high" ||
    value?.environmentShadows === "hybrid" ||
    value?.environmentShadows === "baked" ||
    value?.environmentShadows === "off"
      ? value.environmentShadows
      : "hybrid";
  const carShadow =
    value?.carShadow === "dynamic" ||
    value?.carShadow === "silhouette" ||
    value?.carShadow === "off"
      ? value.carShadow
      : "dynamic";
  return {
    tc: value?.tc !== false,
    tcStrength: clamp(value?.tcStrength ?? 0.5, 0, 1, 1),
    abs: value?.abs !== false,
    esc: value?.esc !== false,
    escStrength: clamp(value?.escStrength ?? 0.5, 0, 1, 1),
    steeringSensitivity: clamp(value?.steeringSensitivity ?? 1, 0.25, 2, 1),
    debugLines: value?.debugLines === true,
    environmentShadows,
    carShadow,
    timeOfDay: clamp(value?.timeOfDay ?? 15, 6, 20, 15),
    antiAliasing:
      value?.antiAliasing === "off" ||
      value?.antiAliasing === "2x" ||
      value?.antiAliasing === "4x"
        ? value.antiAliasing
        : "auto",
    fpsCounter: value?.fpsCounter === true,
    renderScale: RENDER_SCALES.includes(value?.renderScale as RenderScale)
      ? (value!.renderScale as RenderScale)
      : 1,
  };
}

function load(): AssistSettings {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return sanitize(JSON.parse(stored));
    }
    const previous = localStorage.getItem(PREVIOUS_STORAGE_KEY);
    if (previous) {
      const old = JSON.parse(previous) as Partial<AssistSettings> | null;
      if (old && typeof old === "object") {
        return sanitize(old);
      }
    }
    const legacy = LEGACY_STORAGE_KEY ? localStorage.getItem(LEGACY_STORAGE_KEY) : null;
    if (legacy) {
      const old = JSON.parse(legacy) as {
        shadows?: boolean;
        [key: string]: unknown;
      };
      return sanitize({
        ...(old as Partial<AssistSettings>),
        environmentShadows: old.shadows === false ? "off" : "hybrid",
        carShadow: old.shadows === false ? "off" : "dynamic",
      });
    }
    return { ...DEFAULTS };
  } catch {
    return { ...DEFAULTS };
  }
}

function save(settings: AssistSettings): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Storage may be unavailable in private or embedded contexts.
  }
}

const MOBILE_QUERY = "(pointer: coarse), (max-width: 720px)";

function requestedSamples(settings: AssistSettings): number {
  if (settings.antiAliasing === "off") return 1;
  if (settings.antiAliasing === "2x") return 2;
  if (settings.antiAliasing === "4x") return 4;
  return window.matchMedia(MOBILE_QUERY).matches ? 2 : 4;
}

function formatTime(hours: number): string {
  const whole = Math.floor(hours);
  const minutes = Math.round((hours - whole) * 60);
  return `${String(whole).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}

export function setupAssistsDialog(
  viewer: SettingsViewer,
  options: SettingsOptions = {},
): SettingsController | null {
  const gameplay = options.gameplay !== false;
  ensureSettingsUi(gameplay);

  const dialog = document.querySelector<HTMLDialogElement>("#assists-dialog");
  const openButton = document.querySelector<HTMLButtonElement>("#assists-open");
  const closeButton = document.querySelector<HTMLButtonElement>("#assists-close");
  const resetButton = document.querySelector<HTMLButtonElement>("#assists-reset");
  const tcToggle = document.querySelector<HTMLButtonElement>("#tc-toggle");
  const absToggle = document.querySelector<HTMLButtonElement>("#abs-toggle");
  const escToggle = document.querySelector<HTMLButtonElement>("#esc-toggle");
  const tcStrength = document.querySelector<HTMLInputElement>("#tc-strength");
  const escStrength = document.querySelector<HTMLInputElement>("#esc-strength");
  const steering = document.querySelector<HTMLInputElement>("#steering-sensitivity");
  const debugToggle = document.querySelector<HTMLButtonElement>("#debug-lines-toggle");
  const fpsToggle = document.querySelector<HTMLButtonElement>("#fps-counter-toggle");
  const environmentSelect = document.querySelector<HTMLSelectElement>("#environment-shadows");
  const carSelect = document.querySelector<HTMLSelectElement>("#car-shadow");
  const timeRange = document.querySelector<HTMLInputElement>("#time-of-day");
  const timeValue = document.querySelector<HTMLOutputElement>("#time-of-day-value");
  const timeApply = document.querySelector<HTMLButtonElement>("#time-of-day-apply");
  const msaa = document.querySelector<HTMLSelectElement>("#msaa-mode");
  const msaaApplied = document.querySelector<HTMLOutputElement>("#msaa-applied");
  const renderScaleSelect = document.querySelector<HTMLSelectElement>("#render-scale");
  const renderScaleValue = document.querySelector<HTMLOutputElement>("#render-scale-value");
  const tcValue = document.querySelector<HTMLOutputElement>("#tc-strength-value");
  const escValue = document.querySelector<HTMLOutputElement>("#esc-strength-value");
  const steeringValue = document.querySelector<HTMLOutputElement>("#steering-sensitivity-value");
  const status = document.querySelector<HTMLElement>("#assists-status");
  if (
    !dialog ||
    !openButton ||
    !closeButton ||
    !resetButton ||
    !debugToggle ||
    !fpsToggle ||
    !environmentSelect ||
    !carSelect ||
    !timeRange ||
    !timeValue ||
    !timeApply ||
    !msaa ||
    !msaaApplied ||
    !renderScaleSelect ||
    !renderScaleValue ||
    !status
  ) {
    return null;
  }
  if (
    gameplay &&
    (!tcToggle ||
      !absToggle ||
      !escToggle ||
      !tcStrength ||
      !escStrength ||
      !steering ||
      !tcValue ||
      !escValue ||
      !steeringValue)
  ) {
    return null;
  }

  let settings = load();
  let actualMsaa = 1;
  let appliedEnvironment: "high" | "hybrid" | "baked" | "off" = "high";

  const applyAssists = () => {
    if (!gameplay) return;
    viewer.set_traction_control?.(settings.tc);
    viewer.set_traction_strength?.(settings.tcStrength);
    viewer.set_abs_control?.(settings.abs);
    viewer.set_stability_control?.(settings.esc);
    viewer.set_stability_strength?.(settings.escStrength);
    viewer.set_steering_sensitivity?.(settings.steeringSensitivity);
  };

  const applyModes = () => {
    appliedEnvironment = settings.environmentShadows;
    viewer.set_environment_shadow_mode(appliedEnvironment);
    viewer.set_car_shadow_mode(settings.carShadow);
  };

  const apply = () => {
    applyAssists();
    applyModes();
    viewer.set_debug_lines_visible(settings.debugLines);
    setFpsCounterVisible(settings.fpsCounter);
    actualMsaa = viewer.set_msaa_samples(requestedSamples(settings));
    options.onRenderScale?.(settings.renderScale);
    save(settings);
  };

  const refresh = () => {
    if (gameplay && tcToggle && absToggle && escToggle && tcStrength && escStrength && steering && tcValue && escValue && steeringValue) {
      tcToggle.textContent = settings.tc ? "TC: On" : "TC: Off";
      tcToggle.setAttribute("aria-pressed", String(settings.tc));
      absToggle.textContent = settings.abs ? "ABS: On" : "ABS: Off";
      absToggle.setAttribute("aria-pressed", String(settings.abs));
      escToggle.textContent = settings.esc ? "ESC: On" : "ESC: Off";
      escToggle.setAttribute("aria-pressed", String(settings.esc));
      tcStrength.value = String(settings.tcStrength);
      escStrength.value = String(settings.escStrength);
      steering.value = String(settings.steeringSensitivity);
      tcValue.textContent = `${Math.round(settings.tcStrength * 100)}%`;
      escValue.textContent = `${Math.round(settings.escStrength * 100)}%`;
      steeringValue.textContent = `${Math.round(settings.steeringSensitivity * 100)}%`;
    }
    debugToggle.textContent = settings.debugLines ? "Debug: On" : "Debug: Off";
    debugToggle.setAttribute("aria-pressed", String(settings.debugLines));
    fpsToggle!.textContent = settings.fpsCounter ? "FPS: On" : "FPS: Off";
    fpsToggle!.setAttribute("aria-pressed", String(settings.fpsCounter));
    environmentSelect.value = settings.environmentShadows;
    carSelect.value = settings.carShadow;
    timeRange.value = String(settings.timeOfDay);
    timeValue.textContent = formatTime(settings.timeOfDay);
    msaa.value = settings.antiAliasing;
    renderScaleSelect!.value = String(settings.renderScale);
    renderScaleValue!.textContent = `${Math.round(settings.renderScale * 100)}%`;
    const requested = requestedSamples(settings);
    const fallback = requested !== 1 && actualMsaa < requested;
    msaaApplied.textContent =
      settings.antiAliasing === "auto"
        ? `Auto — ${actualMsaa}×${fallback ? " (fallback)" : ""}`
        : actualMsaa === 1 && requested !== 1
          ? `${requested}× unavailable — 1×`
          : `${actualMsaa}×`;
  };

  viewer.set_time_of_day(settings.timeOfDay);
  apply();
  refresh();

  const resetAll = () => {
    settings = { ...DEFAULTS };
    viewer.set_time_of_day(settings.timeOfDay);
    apply();
    refresh();
    try {
      localStorage.removeItem(STORAGE_KEY);
      localStorage.removeItem(PREVIOUS_STORAGE_KEY);
      localStorage.removeItem(LEGACY_STORAGE_KEY);
    } catch {
      // Ignore storage failures.
    }
    status.textContent = "Reset to defaults";
  };

  const controller: SettingsController = {
    get: () => ({ ...settings }),
    update: (patch) => {
      const timeChanged = patch.timeOfDay !== undefined;
      settings = sanitize({ ...settings, ...patch });
      if (timeChanged) {
        viewer.set_time_of_day(settings.timeOfDay);
      }
      apply();
      refresh();
      status.textContent = "Saved";
    },
    refresh,
    reset: resetAll,
  };

  const setTab = (tab: string) => {
    for (const button of dialog.querySelectorAll<HTMLButtonElement>(".settings-tab")) {
      const selected = button.dataset.tab === tab;
      button.setAttribute("aria-selected", String(selected));
    }
    document.querySelector("#settings-gameplay")?.toggleAttribute("hidden", tab !== "gameplay");
    document.querySelector("#settings-graphics")?.toggleAttribute("hidden", tab !== "graphics");
  };
  for (const button of dialog.querySelectorAll<HTMLButtonElement>(".settings-tab")) {
    button.addEventListener("click", () => setTab(button.dataset.tab ?? "gameplay"));
  }
  dialog.querySelector(".settings-tabs")?.addEventListener("keydown", (event) => {
    const keyboard = event as KeyboardEvent;
    if (keyboard.key !== "ArrowLeft" && keyboard.key !== "ArrowRight") return;
    keyboard.preventDefault();
    setTab(
      document
        .querySelector<HTMLButtonElement>(".settings-tab[aria-selected='true']")
        ?.dataset.tab === "gameplay"
        ? "graphics"
        : "gameplay",
    );
  });
  setTab(gameplay ? "gameplay" : "graphics");

  if (gameplay) {
    tcToggle!.addEventListener("click", () => {
      settings.tc = !settings.tc;
      apply();
      refresh();
      status.textContent = "Saved";
    });
    absToggle!.addEventListener("click", () => {
      settings.abs = !settings.abs;
      apply();
      refresh();
      status.textContent = "Saved";
    });
    escToggle!.addEventListener("click", () => {
      settings.esc = !settings.esc;
      apply();
      refresh();
      status.textContent = "Saved";
    });
    tcStrength!.addEventListener("input", () => {
      settings.tcStrength = Number(tcStrength!.value);
      apply();
      refresh();
      status.textContent = "Saved";
    });
    escStrength!.addEventListener("input", () => {
      settings.escStrength = Number(escStrength!.value);
      apply();
      refresh();
      status.textContent = "Saved";
    });
    steering!.addEventListener("input", () => {
      settings.steeringSensitivity = Number(steering!.value);
      apply();
      refresh();
      status.textContent = "Saved";
    });
  }

  environmentSelect.addEventListener("change", () => {
    settings.environmentShadows = environmentSelect.value as EnvironmentShadows;
    applyModes();
    save(settings);
    refresh();
    status.textContent = "Saved";
  });
  carSelect.addEventListener("change", () => {
    settings.carShadow = carSelect.value as CarShadow;
    applyModes();
    save(settings);
    refresh();
    status.textContent = "Saved";
  });
  timeRange.addEventListener("input", () => {
    timeValue.textContent = formatTime(Number(timeRange.value));
  });
  timeApply.addEventListener("click", () => {
    settings.timeOfDay = Number(timeRange.value);
    const bakeMs = viewer.set_time_of_day(settings.timeOfDay);
    save(settings);
    status.textContent =
      bakeMs > 0 ? `Lighting applied · baked in ${bakeMs.toFixed(1)} ms` : "Lighting applied";
  });
  debugToggle.addEventListener("click", () => {
    settings.debugLines = !settings.debugLines;
    viewer.set_debug_lines_visible(settings.debugLines);
    save(settings);
    refresh();
    status.textContent = "Saved";
  });
  fpsToggle.addEventListener("click", () => {
    settings.fpsCounter = !settings.fpsCounter;
    setFpsCounterVisible(settings.fpsCounter);
    save(settings);
    refresh();
    status.textContent = "Saved";
  });
  msaa.addEventListener("change", () => {
    settings.antiAliasing = msaa.value as AntiAliasing;
    actualMsaa = viewer.set_msaa_samples(requestedSamples(settings));
    save(settings);
    refresh();
    status.textContent = "Saved";
  });
  renderScaleSelect.addEventListener("change", () => {
    settings.renderScale = Number(renderScaleSelect.value) as RenderScale;
    apply();
    refresh();
    status.textContent = "Saved";
  });
  resetButton.addEventListener("click", resetAll);
  closeButton.addEventListener("click", () => dialog.close());
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) {
      dialog.close();
    }
  });
  openButton.addEventListener("click", () => {
    refresh();
    status.textContent = "Changes apply live";
    dialog.showModal();
  });
  return controller;
}
