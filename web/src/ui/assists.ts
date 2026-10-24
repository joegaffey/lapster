interface AssistsViewer {
  set_traction_control(enabled: boolean): void;
  set_traction_strength(strength: number): void;
  set_abs_control(enabled: boolean): void;
  set_stability_control(enabled: boolean): void;
  set_stability_strength(strength: number): void;
  set_steering_sensitivity(sensitivity: number): void;
  set_debug_lines_visible(visible: boolean): void;
  set_msaa_samples(requested: number): number;
  msaa_samples(): number;
}

type AntiAliasing = "auto" | "off" | "2x" | "4x";

interface AssistSettings {
  tc: boolean;
  tcStrength: number;
  abs: boolean;
  esc: boolean;
  escStrength: number;
  steeringSensitivity: number;
  debugLines: boolean;
  antiAliasing: AntiAliasing;
}

const STORAGE_KEY = "lapster.assists.v1";

const DEFAULTS: AssistSettings = {
  tc: true,
  tcStrength: 1,
  abs: true,
  esc: true,
  escStrength: 1,
  steeringSensitivity: 1,
  debugLines: false,
  antiAliasing: "auto",
};

function clamp(value: number, min: number, max: number, fallback: number): number {
  return Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;
}

function sanitize(value: Partial<AssistSettings> | null): AssistSettings {
  return {
    tc: value?.tc !== false,
    tcStrength: clamp(value?.tcStrength ?? 1, 0, 1, 1),
    abs: value?.abs !== false,
    esc: value?.esc !== false,
    escStrength: clamp(value?.escStrength ?? 1, 0, 1, 1),
    steeringSensitivity: clamp(value?.steeringSensitivity ?? 1, 0.25, 2, 1),
    debugLines: value?.debugLines === true,
    antiAliasing:
      value?.antiAliasing === "off" ||
      value?.antiAliasing === "2x" ||
      value?.antiAliasing === "4x"
        ? value.antiAliasing
        : "auto",
  };
}

const MOBILE_QUERY = "(pointer: coarse), (max-width: 720px)";

function requestedSamples(settings: AssistSettings): number {
  if (settings.antiAliasing === "off") return 1;
  if (settings.antiAliasing === "2x") return 2;
  if (settings.antiAliasing === "4x") return 4;
  return window.matchMedia(MOBILE_QUERY).matches ? 2 : 4;
}

function load(): AssistSettings {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? sanitize(JSON.parse(stored)) : { ...DEFAULTS };
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

export function setupAssistsDialog(viewer: AssistsViewer): void {
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
  const msaa = document.querySelector<HTMLSelectElement>("#msaa-mode");
  const msaaApplied = document.querySelector<HTMLOutputElement>("#msaa-applied");
  const tcValue = document.querySelector<HTMLOutputElement>("#tc-strength-value");
  const escValue = document.querySelector<HTMLOutputElement>("#esc-strength-value");
  const steeringValue = document.querySelector<HTMLOutputElement>("#steering-sensitivity-value");
  const status = document.querySelector<HTMLElement>("#assists-status");
  if (
    !dialog ||
    !openButton ||
    !closeButton ||
    !resetButton ||
    !tcToggle ||
    !absToggle ||
    !escToggle ||
    !tcStrength ||
    !escStrength ||
    !steering ||
    !debugToggle ||
    !msaa ||
    !msaaApplied ||
    !tcValue ||
    !escValue ||
    !steeringValue ||
    !status
  ) {
    return;
  }

  let settings = load();
  let actualMsaa = 1;

  const apply = () => {
    viewer.set_traction_control(settings.tc);
    viewer.set_traction_strength(settings.tcStrength);
    viewer.set_abs_control(settings.abs);
    viewer.set_stability_control(settings.esc);
    viewer.set_stability_strength(settings.escStrength);
    viewer.set_steering_sensitivity(settings.steeringSensitivity);
    viewer.set_debug_lines_visible(settings.debugLines);
    actualMsaa = viewer.set_msaa_samples(requestedSamples(settings));
    save(settings);
  };

  const refresh = () => {
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
    debugToggle.textContent = settings.debugLines ? "Debug: On" : "Debug: Off";
    debugToggle.setAttribute("aria-pressed", String(settings.debugLines));
    msaa.value = settings.antiAliasing;
    const requested = requestedSamples(settings);
    const fallback = requested !== 1 && actualMsaa < requested;
    msaaApplied.textContent =
      settings.antiAliasing === "auto"
        ? `Auto — ${actualMsaa}×${fallback ? " (fallback)" : ""}`
        : actualMsaa === 1 && requested !== 1
          ? `${requested}× unavailable — 1×`
          : `${actualMsaa}×`;
  };

  apply();
  refresh();

  tcToggle.addEventListener("click", () => {
    settings.tc = !settings.tc;
    apply();
    refresh();
    status.textContent = "Saved";
  });
  absToggle.addEventListener("click", () => {
    settings.abs = !settings.abs;
    apply();
    refresh();
    status.textContent = "Saved";
  });
  escToggle.addEventListener("click", () => {
    settings.esc = !settings.esc;
    apply();
    refresh();
    status.textContent = "Saved";
  });
  tcStrength.addEventListener("input", () => {
    settings.tcStrength = Number(tcStrength.value);
    apply();
    refresh();
    status.textContent = "Saved";
  });
  escStrength.addEventListener("input", () => {
    settings.escStrength = Number(escStrength.value);
    apply();
    refresh();
    status.textContent = "Saved";
  });
  steering.addEventListener("input", () => {
    settings.steeringSensitivity = Number(steering.value);
    apply();
    refresh();
    status.textContent = "Saved";
  });
  debugToggle.addEventListener("click", () => {
    settings.debugLines = !settings.debugLines;
    apply();
    refresh();
    status.textContent = "Saved";
  });
  msaa.addEventListener("change", () => {
    settings.antiAliasing = msaa.value as AntiAliasing;
    apply();
    refresh();
    status.textContent = "Saved";
  });
  resetButton.addEventListener("click", () => {
    settings = { ...DEFAULTS };
    apply();
    refresh();
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // Ignore storage failures.
    }
    status.textContent = "Reset to defaults";
  });
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
}
