import {
  CAR_SETUP_SECTIONS,
  carSetupStorageKey,
  getSetupPath,
  setSetupPath,
  type CarSetupField,
} from "./car-setup.ts";
import type { AssistSettings, SettingsController } from "./assists.ts";

type CameraMode = "chase" | "bonnet" | "cockpit" | "isometric";

interface DevMenuViewer {
  car_setup(): string;
  apply_car_setup(json: string): void;
  reset_car_setup(): void;
  tuning(): string;
  apply_tuning(json: string): void;
  reset_tuning(): void;
}

interface TuningField {
  path: string;
  label: string;
  group: string;
  kind: "float" | "int" | "color" | "bool";
  min: number;
  max: number;
  step: number;
  unit: string;
  apply: "live" | "physics" | "gpu";
}

interface TuningSnapshot {
  values: any;
  schema: TuningField[];
}

interface AudioState {
  enabled: boolean;
  muted: boolean;
  status: string;
}

interface DevMenuOptions {
  carId: string;
  settings: SettingsController;
  getCameraMode(): CameraMode;
  setCameraMode(mode: CameraMode): void;
  getBodyVisible(): boolean;
  setBodyVisible(visible: boolean): void;
  getAudioState(): AudioState;
  toggleAudio(): void;
  toggleAudioMute(): void;
  onCarSetup?(setup: any): void;
  restartPhysics?(tuning: any): Promise<void> | void;
  restartAudio?(tuning: any): Promise<void> | void;
  reloadGpu?(tuning: any): void;
  applyAudio?(tuning: any): void;
}

export interface DevMenu {
  open(): void;
  close(): void;
  toggle(): void;
}

const STYLE_ID = "lapster-dev-menu-style";
const TUNING_STORAGE_KEY = "lapster.tuning.dev.v1";

const STYLE = `
#dev-drawer {
  position: fixed;
  inset: 0 auto 0 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  width: min(360px, 94vw);
  padding: 0;
  border-right: 1px solid #2a303a;
  background: rgba(8, 10, 13, 0.98);
  color: #e6e8eb;
  font: 11px/1.4 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  overflow: hidden;
  visibility: hidden;
  transform: translateX(calc(-100% - 8px));
  transition: transform 180ms ease, visibility 0s linear 180ms;
  box-shadow: 18px 0 48px rgba(0, 0, 0, 0.45);
}
#dev-drawer.dev-open {
  visibility: visible;
  transform: translateX(0);
  transition: transform 180ms ease, visibility 0s;
}
#dev-drawer .dev-head {
  position: relative;
  z-index: 2;
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin: 0;
  padding: 12px;
  border-bottom: 1px solid #2a303a;
  background: rgba(8, 10, 13, 0.98);
}
#dev-drawer .dev-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 12px 12px;
}
#dev-drawer .dev-head h2 {
  margin: 0;
  color: #e6e8eb;
  font-size: 13px;
  letter-spacing: 0;
  text-transform: none;
}
#dev-drawer .dev-badge {
  margin-left: 6px;
  padding: 1px 5px;
  border: 1px solid #38bdf8;
  border-radius: 3px;
  color: #38bdf8;
  font-size: 9px;
  letter-spacing: 0.08em;
}
#dev-open {
  position: fixed;
  left: 0;
  top: 50%;
  z-index: 31;
  min-width: 44px;
  padding: 6px 9px;
  border: 1px solid #3a4350;
  border-left: 0;
  border-radius: 0 4px 4px 0;
  background: rgba(8, 10, 13, 0.92);
  color: #38bdf8;
  font: 10px/1 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  letter-spacing: 0.12em;
  opacity: 0.82;
  transform: translateY(-50%);
  transition: transform 180ms ease, opacity 180ms ease, visibility 0s linear 0s;
  cursor: pointer;
}
#dev-open:hover { background: #151a21; opacity: 1; }
body.dev-menu-open #dev-open {
  visibility: hidden;
  opacity: 0;
  pointer-events: none;
  transform: translate(-100%, -50%);
  transition: transform 180ms ease, opacity 180ms ease, visibility 0s linear 180ms;
}
#dev-drawer button,
#dev-drawer select,
#dev-drawer input[type="text"] {
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: #151a21;
  color: #e6e8eb;
  padding: 4px 6px;
  font: inherit;
  font-size: 10px;
}
#dev-drawer button { cursor: pointer; }
#dev-drawer button:hover { background: #1c2430; }
#dev-drawer button:disabled { cursor: default; opacity: 0.55; }
#dev-drawer details {
  border-bottom: 1px solid #202630;
  padding: 0 0 8px;
}
#dev-drawer details > summary {
  padding: 9px 0 6px;
  color: #8f9bab;
  cursor: pointer;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  user-select: none;
}
#dev-drawer details details {
  border-bottom: 1px solid #202630;
  padding: 0 0 6px 8px;
}
#dev-drawer details details:last-child { border-bottom: 0; }
#dev-drawer details details > summary {
  padding: 7px 0 5px;
  color: #6f7b8b;
  letter-spacing: 0.04em;
  text-transform: none;
}
#dev-drawer .dev-grid {
  display: grid;
  gap: 6px;
}
#dev-drawer .dev-field {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 4px 8px;
}
#dev-drawer .dev-field > span { min-width: 0; }
#dev-drawer .dev-field output {
  color: #8f9bab;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
#dev-drawer .dev-field input[type="range"] {
  grid-column: 1 / -1;
  width: 100%;
  height: 16px;
  margin: 0;
  accent-color: #38bdf8;
}
#dev-drawer .dev-field select,
#dev-drawer .dev-field input[type="text"] {
  grid-column: 1 / -1;
  width: 100%;
  min-width: 0;
}
#dev-drawer .dev-field input[type="color"] {
  grid-column: 1 / -1;
  width: 100%;
  height: 22px;
  padding: 1px;
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: #151a21;
}
#dev-drawer .dev-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
#dev-drawer .dev-toggle input {
  width: 14px;
  height: 14px;
  margin: 0;
  accent-color: #38bdf8;
}
#dev-drawer .dev-footer {
  position: relative;
  z-index: 3;
  display: grid;
  flex: none;
  gap: 6px;
  margin: 0;
  padding: 10px 12px calc(10px + env(safe-area-inset-bottom));
  border-top: 1px solid #2a303a;
  background: rgba(8, 10, 13, 0.98);
}
#dev-drawer .dev-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
#dev-drawer .dev-status {
  margin: 0;
  color: #8f9bab;
  font-size: 10px;
  line-height: 1.4;
}
#dev-drawer .dev-status.error { color: #f87171; }
#dev-drawer .dev-note {
  margin: 0;
  padding: 0 0 4px;
  color: #6f7b8b;
  font-size: 10px;
  line-height: 1.35;
}
#dev-drawer .dev-audio {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
#dev-drawer .dev-audio span {
  flex: 1;
  color: #8f9bab;
}
@media (min-width: 900px) {
  body.dev-menu-open #hud {
    left: 376px;
    width: min(400px, calc(100vw - 408px));
  }
  body.dev-menu-open #fps-counter { left: 376px; }
}
#dev-drawer .dev-subhead {
  margin: 8px 0 2px;
  color: #cbd5e1;
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
`;

function formatNumber(value: number, step = 0.01): string {
  if (!Number.isFinite(value)) return "0";
  if (step >= 1) return String(Math.round(value));
  return String(Number(value.toFixed(4)));
}

function getValue(target: any, path: string): any {
  let current = target;
  for (const key of path.split(".")) {
    current = current?.[key];
  }
  return current;
}

function setValue(target: any, path: string, value: any): void {
  const keys = path.split(".");
  let current = target;
  for (const key of keys.slice(0, -1)) {
    current = current[key];
  }
  current[keys[keys.length - 1]] = value;
}

function colorToHex(value: number[]): string {
  const channel = (index: number) =>
    Math.round(Math.min(1, Math.max(0, value?.[index] ?? 0)) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${channel(0)}${channel(1)}${channel(2)}`;
}

function hexToColor(value: string): [number, number, number] {
  const match = /^#?([0-9a-f]{6})$/i.exec(value);
  const hex = match?.[1] ?? "ffffff";
  return [
    parseInt(hex.slice(0, 2), 16) / 255,
    parseInt(hex.slice(2, 4), 16) / 255,
    parseInt(hex.slice(4, 6), 16) / 255,
  ];
}

function readTuning(viewer: DevMenuViewer): TuningSnapshot {
  const parsed = JSON.parse(viewer.tuning());
  return {
    values: parsed?.values ?? {},
    schema: Array.isArray(parsed?.schema) ? parsed.schema : [],
  };
}

function fieldRow(labelText: string, control: HTMLElement, output?: HTMLElement): HTMLLabelElement {
  const label = document.createElement("label");
  label.className = "dev-field";
  const name = document.createElement("span");
  name.textContent = labelText;
  label.append(name);
  if (output) {
    label.append(output);
  } else {
    const spacer = document.createElement("span");
    label.append(spacer);
  }
  label.append(control);
  return label;
}

function toggleRow(labelText: string, input: HTMLInputElement): HTMLLabelElement {
  const label = document.createElement("label");
  label.className = "dev-toggle";
  const name = document.createElement("span");
  name.textContent = labelText;
  label.append(name, input);
  return label;
}

function addSection(host: HTMLElement, title: string): HTMLDetailsElement {
  const details = document.createElement("details");
  const summary = document.createElement("summary");
  summary.textContent = title;
  const grid = document.createElement("div");
  grid.className = "dev-grid";
  details.append(summary, grid);
  host.append(details);
  return details;
}

export function setupDevMenu(viewer: DevMenuViewer, options: DevMenuOptions): DevMenu {
  if (!document.querySelector(`#${STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.append(style);
  }

  const openButton = document.createElement("button");
  openButton.id = "dev-open";
  openButton.type = "button";
  openButton.textContent = "DEV";
  openButton.title = "Open developer settings";
  openButton.setAttribute("aria-label", "Open developer settings");
  openButton.setAttribute("aria-controls", "dev-drawer");
  openButton.setAttribute("aria-expanded", "false");
  document.body.append(openButton);

  const drawer = document.createElement("aside");
  drawer.id = "dev-drawer";
  drawer.setAttribute("aria-label", "Developer settings");
  drawer.setAttribute("aria-hidden", "true");

  const head = document.createElement("header");
  head.className = "dev-head";
  const title = document.createElement("h2");
  title.innerHTML = 'Developer settings <span class="dev-badge">DEV</span>';
  const closeButton = document.createElement("button");
  closeButton.id = "dev-close";
  closeButton.type = "button";
  closeButton.textContent = "Close";
  head.append(title, closeButton);
  drawer.append(head);
  const content = document.createElement("div");
  content.className = "dev-body";
  drawer.append(content);

  const status = document.createElement("p");
  status.id = "dev-status";
  status.className = "dev-status";
  status.setAttribute("aria-live", "polite");
  status.textContent = "Changes apply live";

  let current = options.settings.get();
  let carSetup: any = JSON.parse(viewer.car_setup());
  let carApplyTimer = 0;
  let audioRefreshTimer = 0;
  let tuningApplyTimer = 0;
  let tuningValues: any = {};
  let tuningSchema: TuningField[] = [];
  const tuningSnapshot = readTuning(viewer);
  tuningValues = tuningSnapshot.values;
  tuningSchema = tuningSnapshot.schema;
  const pending = { physics: new Set<string>(), gpu: new Set<string>() };

  const refreshSettings = () => {
    current = options.settings.get();
    tc.checked = current.tc;
    tcStrength.value = String(current.tcStrength);
    tcStrengthValue.textContent = `${Math.round(current.tcStrength * 100)}%`;
    abs.checked = current.abs;
    esc.checked = current.esc;
    autoGears.checked = current.autoGears;
    autoClutch.checked = current.autoClutch;
    escStrength.value = String(current.escStrength);
    escStrengthValue.textContent = `${Math.round(current.escStrength * 100)}%`;
    steering.value = String(current.steeringSensitivity);
    steeringValue.textContent = `${Math.round(current.steeringSensitivity * 100)}%`;
    debug.checked = current.debugLines;
    fps.checked = current.fpsCounter;
    environment.value = current.environmentShadows;
    carShadow.value = current.carShadow;
    time.value = String(current.timeOfDay);
    timeValue.textContent = formatNumber(current.timeOfDay, 0.5);
    antiAliasing.value = current.antiAliasing;
    renderScale.value = String(current.renderScale);
    renderScaleValue.textContent = `${Math.round(current.renderScale * 100)}%`;
  };

  const updateSetting = (patch: Partial<AssistSettings>) => {
    options.settings.update(patch);
    refreshSettings();
    status.className = "dev-status";
    status.textContent = "Saved";
  };

  const settingsBody = addSection(content, "Driving assists").querySelector(".dev-grid")!;
  const tc = document.createElement("input");
  tc.id = "dev-tc";
  tc.type = "checkbox";
  tc.addEventListener("change", () => updateSetting({ tc: tc.checked }));
  settingsBody.append(toggleRow("Traction control", tc));

  const tcStrengthValue = document.createElement("output");
  const tcStrength = document.createElement("input");
  tcStrength.id = "dev-tc-strength";
  tcStrength.type = "range";
  tcStrength.min = "0";
  tcStrength.max = "1";
  tcStrength.step = "0.05";
  tcStrength.addEventListener("input", () => {
    tcStrengthValue.textContent = `${Math.round(Number(tcStrength.value) * 100)}%`;
    updateSetting({ tcStrength: Number(tcStrength.value) });
  });
  settingsBody.append(fieldRow("TC strength", tcStrength, tcStrengthValue));

  const abs = document.createElement("input");
  abs.id = "dev-abs";
  abs.type = "checkbox";
  abs.addEventListener("change", () => updateSetting({ abs: abs.checked }));
  settingsBody.append(toggleRow("Anti-lock brakes", abs));

  const esc = document.createElement("input");
  esc.id = "dev-esc";
  esc.type = "checkbox";
  esc.addEventListener("change", () => updateSetting({ esc: esc.checked }));
  settingsBody.append(toggleRow("Stability control", esc));

  const autoGears = document.createElement("input");
  autoGears.id = "dev-auto-gears";
  autoGears.type = "checkbox";
  autoGears.addEventListener("change", () => updateSetting({ autoGears: autoGears.checked }));
  settingsBody.append(toggleRow("Automatic gearbox", autoGears));

  const autoClutch = document.createElement("input");
  autoClutch.id = "dev-auto-clutch";
  autoClutch.type = "checkbox";
  autoClutch.addEventListener("change", () => updateSetting({ autoClutch: autoClutch.checked }));
  settingsBody.append(toggleRow("Automatic clutch", autoClutch));

  const escStrengthValue = document.createElement("output");
  const escStrength = document.createElement("input");
  escStrength.id = "dev-esc-strength";
  escStrength.type = "range";
  escStrength.min = "0";
  escStrength.max = "1";
  escStrength.step = "0.05";
  escStrength.addEventListener("input", () => {
    escStrengthValue.textContent = `${Math.round(Number(escStrength.value) * 100)}%`;
    updateSetting({ escStrength: Number(escStrength.value) });
  });
  settingsBody.append(fieldRow("ESC strength", escStrength, escStrengthValue));

  const steeringValue = document.createElement("output");
  const steering = document.createElement("input");
  steering.id = "dev-steering-sensitivity";
  steering.type = "range";
  steering.min = "0.25";
  steering.max = "2";
  steering.step = "0.05";
  steering.addEventListener("input", () => {
    steeringValue.textContent = `${Math.round(Number(steering.value) * 100)}%`;
    updateSetting({ steeringSensitivity: Number(steering.value) });
  });
  settingsBody.append(fieldRow("Steering sensitivity", steering, steeringValue));

  const viewBody = addSection(content, "View").querySelector(".dev-grid")!;
  const camera = document.createElement("select");
  camera.id = "dev-camera-mode";
  for (const mode of ["chase", "bonnet", "cockpit", "isometric"] as CameraMode[]) {
    const option = document.createElement("option");
    option.value = mode;
    option.textContent = mode[0].toUpperCase() + mode.slice(1);
    camera.append(option);
  }
  camera.addEventListener("change", () => {
    options.setCameraMode(camera.value as CameraMode);
    refreshView();
  });
  viewBody.append(fieldRow("Camera", camera));

  const bodyVisible = document.createElement("input");
  bodyVisible.id = "dev-body-visible";
  bodyVisible.type = "checkbox";
  bodyVisible.addEventListener("change", () => {
    options.setBodyVisible(bodyVisible.checked);
    refreshView();
  });
  viewBody.append(toggleRow("Body visible", bodyVisible));

  const debug = document.createElement("input");
  debug.id = "dev-debug-lines";
  debug.type = "checkbox";
  debug.addEventListener("change", () => updateSetting({ debugLines: debug.checked }));
  viewBody.append(toggleRow("Debug overlay", debug));

  const fps = document.createElement("input");
  fps.id = "dev-fps-counter";
  fps.type = "checkbox";
  fps.addEventListener("change", () => updateSetting({ fpsCounter: fps.checked }));
  viewBody.append(toggleRow("FPS counter", fps));

  const antiAliasing = document.createElement("select");
  antiAliasing.id = "dev-anti-aliasing";
  for (const [value, label] of [
    ["auto", "Auto"],
    ["off", "Off"],
    ["2x", "2× MSAA"],
    ["4x", "4× MSAA"],
  ]) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    antiAliasing.append(option);
  }
  antiAliasing.addEventListener("change", () => {
    updateSetting({ antiAliasing: antiAliasing.value as AssistSettings["antiAliasing"] });
  });
  viewBody.append(fieldRow("Anti-aliasing", antiAliasing));

  const renderScaleValue = document.createElement("output");
  const renderScale = document.createElement("select");
  renderScale.id = "dev-render-scale";
  for (const scale of [1, 0.85, 0.75, 0.67, 0.5]) {
    const option = document.createElement("option");
    option.value = String(scale);
    option.textContent = `${Math.round(scale * 100)}%`;
    renderScale.append(option);
  }
  renderScale.addEventListener("change", () => {
    updateSetting({ renderScale: Number(renderScale.value) as AssistSettings["renderScale"] });
  });
  viewBody.append(fieldRow("Render scale", renderScale, renderScaleValue));

  const environment = document.createElement("select");
  environment.id = "dev-environment-shadows";
  for (const [value, label] of [
    ["baked", "Static"],
    ["hybrid", "Static + near dynamic"],
    ["high", "Dynamic"],
    ["off", "Off"],
  ]) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    environment.append(option);
  }
  environment.addEventListener("change", () => {
    updateSetting({ environmentShadows: environment.value as AssistSettings["environmentShadows"] });
  });
  viewBody.append(fieldRow("Environment shadows", environment));

  const carShadow = document.createElement("select");
  carShadow.id = "dev-car-shadow";
  for (const [value, label] of [
    ["dynamic", "Dynamic"],
    ["silhouette", "Silhouette"],
    ["off", "Off"],
  ]) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    carShadow.append(option);
  }
  carShadow.addEventListener("change", () => {
    updateSetting({ carShadow: carShadow.value as AssistSettings["carShadow"] });
  });
  viewBody.append(fieldRow("Car shadow", carShadow));

  const timeValue = document.createElement("output");
  const time = document.createElement("input");
  time.id = "dev-time-of-day";
  time.type = "range";
  time.min = "6";
  time.max = "20";
  time.step = "0.5";
  time.addEventListener("input", () => {
    timeValue.textContent = formatNumber(Number(time.value), 0.5);
  });
  time.addEventListener("change", () => updateSetting({ timeOfDay: Number(time.value) }));
  viewBody.append(fieldRow("Time of day", time, timeValue));

  const audioBody = addSection(content, "Audio").querySelector(".dev-grid")!;
  const audioLine = document.createElement("div");
  audioLine.className = "dev-audio";
  const audioState = document.createElement("span");
  const audioToggle = document.createElement("button");
  audioToggle.id = "dev-audio-toggle";
  audioToggle.type = "button";
  audioToggle.addEventListener("click", () => {
    options.toggleAudio();
    window.setTimeout(refreshAudio, 80);
  });
  const audioMute = document.createElement("button");
  audioMute.id = "dev-audio-mute";
  audioMute.type = "button";
  audioMute.addEventListener("click", () => {
    options.toggleAudioMute();
    window.setTimeout(refreshAudio, 80);
  });
  audioLine.append(audioState, audioToggle, audioMute);
  audioBody.append(audioLine);

  const tuningControls = new Map<
    string,
    { input: HTMLInputElement; output?: HTMLOutputElement; field: TuningField }
  >();

  const persistTuning = () => {
    try {
      localStorage.setItem(TUNING_STORAGE_KEY, JSON.stringify(tuningValues));
    } catch {
      // Storage may be unavailable in private or embedded contexts.
    }
  };

  const applyTuningNow = () => {
    tuningApplyTimer = 0;
    try {
      viewer.apply_tuning(JSON.stringify(tuningValues));
      options.applyAudio?.(tuningValues.audio);
      persistTuning();
      status.className = "dev-status";
      status.textContent = "Tuning applied";
    } catch (error) {
      status.className = "dev-status error";
      status.textContent = error instanceof Error ? error.message : String(error);
    }
  };

  const scheduleTuning = () => {
    status.className = "dev-status";
    status.textContent = "Editing tuning…";
    window.clearTimeout(tuningApplyTimer);
    tuningApplyTimer = window.setTimeout(applyTuningNow, 40);
  };

  const tuningValueText = (field: TuningField, value: number) =>
    `${formatNumber(value, field.step)}${field.unit ? ` ${field.unit}` : ""}`;

  const refreshTuning = () => {
    const snapshot = readTuning(viewer);
    tuningValues = snapshot.values;
    tuningSchema = snapshot.schema;
    for (const [path, control] of tuningControls) {
      const value = getValue(tuningValues, path);
      if (control.field.kind === "color") {
        control.input.value = colorToHex(value);
      } else if (control.field.kind === "bool") {
        control.input.checked = value === true;
      } else {
        control.input.value = String(value);
        if (control.output) {
          control.output.textContent = tuningValueText(control.field, Number(value));
        }
      }
    }
  };

  const tuningTree = new Map<string, Map<string, TuningField[]>>();
  for (const field of tuningSchema) {
    const [parent, child = "General"] = field.group.split(" / ");
    const children = tuningTree.get(parent) ?? new Map<string, TuningField[]>();
    const group = children.get(child) ?? [];
    group.push(field);
    children.set(child, group);
    tuningTree.set(parent, children);
  }
  for (const [parent, children] of tuningTree) {
    const parentDetails = addSection(content, parent);
    const parentBody = parentDetails.querySelector(".dev-grid")!;
    for (const [child, fields] of children) {
      const section = addSection(parentBody, child);
      section.classList.add("dev-subsection");
      const grid = section.querySelector(".dev-grid")!;
      if (parent === "GPU") {
        const note = document.createElement("p");
        note.className = "dev-note";
        note.textContent =
          "With Environment shadows set to Dynamic, the environment and car share the dynamic map. The car-only map is used when environment shadows are Static/Baked. Static environment shadows use baked resolution; a smaller baked region gives sharper shadows near spawn but loses coverage farther away.";
        grid.append(note);
      } else if (parent === "Rendering" && child === "Shadows") {
        const note = document.createElement("p");
        note.className = "dev-note";
        note.textContent =
          "Full-track dynamic shadows require Environment shadows = Dynamic. They use the whole track as the shadow region and are more expensive.";
        grid.append(note);
      }
      for (const field of fields) {
      const id = `dev-tune-${field.path.replace(/[^a-zA-Z0-9]/g, "-")}`;
      if (field.kind === "bool") {
        const input = document.createElement("input");
        input.id = id;
        input.type = "checkbox";
        input.checked = getValue(tuningValues, field.path) === true;
        input.addEventListener("change", () => {
          setValue(tuningValues, field.path, input.checked);
          scheduleTuning();
        });
        grid.append(toggleRow(field.label, input));
        tuningControls.set(field.path, { input, field });
      } else if (field.kind === "color") {
        const input = document.createElement("input");
        input.id = id;
        input.type = "color";
        input.value = colorToHex(getValue(tuningValues, field.path));
        input.addEventListener("input", () => {
          setValue(tuningValues, field.path, hexToColor(input.value));
          scheduleTuning();
        });
        grid.append(fieldRow(field.label, input));
        tuningControls.set(field.path, { input, field });
      } else {
        const output = document.createElement("output");
        const input = document.createElement("input");
        input.id = id;
        input.type = "range";
        input.min = String(field.min);
        input.max = String(field.max);
        input.step = String(field.kind === "int" ? Math.max(1, field.step) : field.step);
        const value = Number(getValue(tuningValues, field.path));
        input.value = String(value);
        output.textContent = tuningValueText(field, value);
        input.addEventListener("input", () => {
          const next =
            field.kind === "int" ? Math.round(Number(input.value)) : Number(input.value);
          setValue(tuningValues, field.path, next);
          output.textContent = tuningValueText(field, next);
          if (field.apply !== "live") {
            pending[field.apply].add(field.path);
            updateRestartButtons();
          }
          scheduleTuning();
        });
        grid.append(fieldRow(field.label, input, output));
        tuningControls.set(field.path, { input, output, field });
      }
      }
    }
  }

  const setupControls = new Map<string, HTMLInputElement>();
  const setupOutputs = new Map<string, HTMLOutputElement>();

  const refreshView = () => {
    camera.value = options.getCameraMode();
    bodyVisible.checked = options.getBodyVisible();
  };

  const refreshAudio = () => {
    window.clearTimeout(audioRefreshTimer);
    const state = options.getAudioState();
    audioToggle.disabled = state.status === "unavailable";
    audioMute.disabled = state.status === "unavailable";
    audioToggle.textContent = state.enabled ? "Audio on" : "Audio off";
    audioMute.textContent = state.muted ? "Unmute" : "Mute";
    audioState.textContent =
      state.status === "unavailable"
        ? "Audio unavailable"
        : state.status === "starting"
          ? "Audio starting…"
          : state.status === "error"
            ? "Audio error"
            : state.status === "running"
              ? state.muted
                ? "Running · muted"
                : "Running"
              : state.enabled
                ? "Ready"
                : "Off";
    if (state.status === "starting") {
      audioRefreshTimer = window.setTimeout(refreshAudio, 150);
    }
  };

  const persistCarSetup = () => {
    try {
      localStorage.setItem(carSetupStorageKey(options.carId), JSON.stringify(carSetup));
    } catch {
      // Storage may be unavailable in private or embedded contexts.
    }
  };

  const applyCarSetup = () => {
    carApplyTimer = 0;
    try {
      viewer.apply_car_setup(JSON.stringify(carSetup));
      const applied = JSON.parse(viewer.car_setup());
      options.onCarSetup?.(applied);
      persistCarSetup();
      status.className = "dev-status";
      status.textContent = "Car setup applied";
    } catch (error) {
      status.className = "dev-status error";
      status.textContent = error instanceof Error ? error.message : String(error);
    }
  };

  const scheduleCarSetup = () => {
    status.className = "dev-status";
    status.textContent = "Editing car setup…";
    window.clearTimeout(carApplyTimer);
    carApplyTimer = window.setTimeout(applyCarSetup, 60);
  };

  const refreshCarSetup = () => {
    carSetup = JSON.parse(viewer.car_setup());
    for (const [path, input] of setupControls) {
      const field = findSetupField(path);
      if (!field) continue;
      if (field.kind === "json") {
        input.value = JSON.stringify(getSetupPath(carSetup, path));
      } else {
        const factor = field.factor ?? 1;
        input.value = formatNumber(Number(getSetupPath(carSetup, path)) * factor, field.step);
        const output = setupOutputs.get(path);
        if (output) {
          output.textContent = `${input.value}${field.unit ? ` ${field.unit}` : ""}`;
        }
      }
    }
  };

  const carTuning = addSection(content, "Car tuning");
  const carTuningBody = carTuning.querySelector(".dev-grid")!;
  for (const section of CAR_SETUP_SECTIONS) {
    const details = addSection(carTuningBody, section.title);
    details.classList.add("dev-subsection");
    const grid = details.querySelector(".dev-grid")!;
    for (const field of section.fields) {
      if (field.kind === "json") {
        const input = document.createElement("input");
        input.type = "text";
        input.id = `dev-setup-${field.path.replace(/[^a-zA-Z0-9]/g, "-")}`;
        input.spellcheck = false;
        input.addEventListener("change", () => {
          try {
            const parsed = JSON.parse(input.value);
            if (!Array.isArray(parsed)) {
              throw new Error(`${field.label} must be a JSON array`);
            }
            setSetupPath(carSetup, field.path, parsed);
            scheduleCarSetup();
          } catch (error) {
            status.className = "dev-status error";
            status.textContent = error instanceof Error ? error.message : String(error);
          }
        });
        grid.append(fieldRow(field.label, input));
        setupControls.set(field.path, input);
      } else {
        const output = document.createElement("output");
        const input = document.createElement("input");
        input.id = `dev-setup-${field.path.replace(/[^a-zA-Z0-9]/g, "-")}`;
        input.type = "range";
        input.min = String(field.min ?? 0);
        input.max = String(field.max ?? 0);
        input.step = String(field.step ?? 0.01);
        input.addEventListener("input", () => {
          const factor = field.factor ?? 1;
          setSetupPath(carSetup, field.path, Number(input.value) / factor);
          output.textContent = `${input.value}${field.unit ? ` ${field.unit}` : ""}`;
          scheduleCarSetup();
        });
        grid.append(fieldRow(field.label, input, output));
        setupControls.set(field.path, input);
        setupOutputs.set(field.path, output);
      }
    }
  }

  const restartPhysics = document.createElement("button");
  restartPhysics.type = "button";
  restartPhysics.hidden = true;
  restartPhysics.addEventListener("click", async () => {
    window.clearTimeout(tuningApplyTimer);
    tuningApplyTimer = 0;
    applyTuningNow();
    try {
      await options.restartPhysics?.(tuningValues.physics);
      pending.physics.clear();
      updateRestartButtons();
      status.className = "dev-status";
      status.textContent = "Physics restarted";
    } catch (error) {
      status.className = "dev-status error";
      status.textContent = error instanceof Error ? error.message : String(error);
    }
  });
  const restartAudio = document.createElement("button");
  restartAudio.type = "button";
  restartAudio.textContent = "Restart audio";
  restartAudio.addEventListener("click", async () => {
    try {
      options.applyAudio?.(tuningValues.audio);
      await options.restartAudio?.(tuningValues.audio);
      refreshAudio();
      status.className = "dev-status";
      status.textContent = "Audio restarted";
    } catch (error) {
      status.className = "dev-status error";
      status.textContent = error instanceof Error ? error.message : String(error);
    }
  });
  const reloadGpu = document.createElement("button");
  reloadGpu.type = "button";
  reloadGpu.hidden = true;
  reloadGpu.addEventListener("click", () => {
    window.clearTimeout(tuningApplyTimer);
    tuningApplyTimer = 0;
    applyTuningNow();
    pending.gpu.clear();
    updateRestartButtons();
    options.reloadGpu?.(tuningValues.gpu);
  });
  const restartActions = document.createElement("div");
  restartActions.className = "dev-actions dev-restarts";
  restartActions.append(restartPhysics, restartAudio, reloadGpu);

  function updateRestartButtons() {
    restartPhysics.hidden = pending.physics.size === 0;
    restartPhysics.textContent = `Apply physics (${pending.physics.size})`;
    reloadGpu.hidden = pending.gpu.size === 0;
    reloadGpu.textContent = `Apply GPU & reload (${pending.gpu.size})`;
  }

  const actions = document.createElement("div");
  actions.className = "dev-actions";
  const resetSettings = document.createElement("button");
  resetSettings.type = "button";
  resetSettings.textContent = "Reset settings";
  resetSettings.addEventListener("click", () => {
    options.settings.reset();
    refreshSettings();
    status.className = "dev-status";
    status.textContent = "Settings reset";
  });
  const resetTuning = document.createElement("button");
  resetTuning.type = "button";
  resetTuning.textContent = "Reset tuning";
  resetTuning.addEventListener("click", () => {
    window.clearTimeout(tuningApplyTimer);
    tuningApplyTimer = 0;
    try {
      viewer.reset_tuning();
      try {
        localStorage.removeItem(TUNING_STORAGE_KEY);
      } catch {
        // Ignore storage failures.
      }
      pending.physics.add("reset");
      pending.gpu.add("reset");
      updateRestartButtons();
      refreshTuning();
      options.applyAudio?.(readTuning(viewer).values.audio);
      status.className = "dev-status";
      status.textContent = "Tuning reset";
    } catch (error) {
      status.className = "dev-status error";
      status.textContent = error instanceof Error ? error.message : String(error);
    }
  });
  const resetCar = document.createElement("button");
  resetCar.type = "button";
  resetCar.textContent = "Reset car";
  resetCar.addEventListener("click", () => {
    try {
      viewer.reset_car_setup();
      try {
        localStorage.removeItem(carSetupStorageKey(options.carId));
      } catch {
        // Ignore storage failures.
      }
      options.onCarSetup?.(JSON.parse(viewer.car_setup()));
      refreshCarSetup();
      status.className = "dev-status";
      status.textContent = "Car reset";
    } catch (error) {
      status.className = "dev-status error";
      status.textContent = error instanceof Error ? error.message : String(error);
    }
  });
  const footer = document.createElement("div");
  footer.className = "dev-footer";
  actions.append(resetSettings, resetTuning, resetCar);
  footer.append(restartActions, actions, status);
  drawer.append(footer);

  document.body.append(drawer);

  const isOpen = () => drawer.classList.contains("dev-open");
  const open = () => {
    refreshSettings();
    refreshView();
    refreshAudio();
    refreshTuning();
    refreshCarSetup();
    document.body.classList.add("dev-menu-open");
    drawer.classList.add("dev-open");
    drawer.setAttribute("aria-hidden", "false");
    openButton.setAttribute("aria-expanded", "true");
    status.className = "dev-status";
    status.textContent = "Changes apply live";
  };
  const close = () => {
    document.body.classList.remove("dev-menu-open");
    drawer.classList.remove("dev-open");
    drawer.setAttribute("aria-hidden", "true");
    openButton.setAttribute("aria-expanded", "false");
  };
  const toggle = () => {
    if (isOpen()) close();
    else open();
  };

  openButton.addEventListener("click", toggle);
  closeButton.addEventListener("click", close);
  window.addEventListener("keydown", (event) => {
    const target = event.target;
    if (
      target instanceof HTMLElement &&
      (target.tagName === "INPUT" ||
        target.tagName === "SELECT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable)
    ) {
      return;
    }
    if (event.key === "\\") {
      event.preventDefault();
      toggle();
    } else if (event.key === "Escape" && isOpen()) {
      event.preventDefault();
      close();
    }
  });

  return { open, close, toggle };
}

function findSetupField(path: string): CarSetupField | undefined {
  for (const section of CAR_SETUP_SECTIONS) {
    const field = section.fields.find((candidate) => candidate.path === path);
    if (field) return field;
  }
  return undefined;
}
