import { APP_VERSION } from "../version.generated.ts";

const SETUP_SCHEMA = "lapster/setup@1";

interface SetupViewer {
  car_setup(): string;
  default_car_setup(): string;
  apply_car_setup(json: string): void;
  reset_car_setup(): void;
}

interface Field {
  path: string;
  label: string;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  kind?: "number" | "json";
  factor?: number;
}

interface Section {
  title: string;
  open?: boolean;
  fields: Field[];
}

const number = (
  path: string,
  label: string,
  min: number,
  max: number,
  step: number,
  unit?: string,
): Field => ({ path, label, min, max, step, unit });

const json = (path: string, label: string): Field => ({ path, label, kind: "json" });

const axleSections = (axle: "front" | "rear", title: string): Section[] => {
  const prefix = `${axle}`;
  return [
    {
      title: `${title} suspension`,
      fields: [
        number(`${prefix}.suspension.travel`, "Travel", 0.02, 0.5, 0.01, "m"),
        number(`${prefix}.suspension.springRate`, "Spring rate", 5_000, 300_000, 1_000, "N/m"),
        number(`${prefix}.suspension.damperBump`, "Bump damping", 500, 50_000, 100, "N·s/m"),
        number(`${prefix}.suspension.damperRebound`, "Rebound damping", 500, 50_000, 100, "N·s/m"),
        number(`${prefix}.suspension.antiRoll`, "Anti-roll", 0, 100_000, 500, "N/m"),
      ],
    },
    {
      title: `${title} tire`,
      fields: [
        number(`${prefix}.tire.lateral.b`, "Lateral B", 0.5, 30, 0.1),
        number(`${prefix}.tire.lateral.c`, "Lateral C", 0.5, 3, 0.01),
        number(`${prefix}.tire.lateral.d`, "Lateral D", 0.1, 2.5, 0.01),
        number(`${prefix}.tire.lateral.e`, "Lateral E", 0, 2, 0.01),
        number(`${prefix}.tire.longitudinal.b`, "Longitudinal B", 0.5, 30, 0.1),
        number(`${prefix}.tire.longitudinal.c`, "Longitudinal C", 0.5, 3, 0.01),
        number(`${prefix}.tire.longitudinal.d`, "Longitudinal D", 0.1, 2.5, 0.01),
        number(`${prefix}.tire.longitudinal.e`, "Longitudinal E", 0, 2, 0.01),
        number(`${prefix}.tire.relaxationLength`, "Relaxation length", 0.01, 3, 0.01, "m"),
        number(`${prefix}.tire.peakSlipRatio`, "Peak slip ratio", 0.01, 1, 0.01),
        number(`${prefix}.tire.frictionScale`, "Friction scale", 0.1, 2, 0.01),
      ],
    },
  ];
};

const SECTIONS: Section[] = [
  {
    title: "Chassis",
    open: true,
    fields: [
      number("chassis.inertia.0", "Inertia X", 50, 10_000, 10, "kg·m²"),
      number("chassis.inertia.1", "Inertia Y", 50, 10_000, 10, "kg·m²"),
      number("chassis.inertia.2", "Inertia Z", 50, 10_000, 10, "kg·m²"),
      number("chassis.centerOfGravity.0", "CG X", -2, 2, 0.01, "m"),
      number("chassis.centerOfGravity.1", "CG height", -2, 2, 0.01, "m"),
      number("chassis.centerOfGravity.2", "CG Z", -2, 2, 0.01, "m"),
    ],
  },
  ...axleSections("front", "Front"),
  ...axleSections("rear", "Rear"),
  {
    title: "Engine",
    fields: [
      number("engine.idleRpm", "Idle", 400, 2_500, 50, "rpm"),
      number("engine.redlineRpm", "Redline", 1_500, 15_000, 100, "rpm"),
      json("engine.torqueCurve", "Torque curve [[rpm,Nm],…]"),
    ],
  },
  {
    title: "Drivetrain",
    fields: [
      number("drivetrain.finalDrive", "Final drive", 1, 8, 0.01, ":1"),
      json("drivetrain.gearRatios", "Gear ratios [..]"),
    ],
  },
  {
    title: "Brakes",
    fields: [
      number("brakes.front", "Front torque", 100, 10_000, 50, "Nm"),
      number("brakes.rear", "Rear torque", 100, 10_000, 50, "Nm"),
    ],
  },
  {
    title: "Steering",
    fields: [
      {
        ...number("steering.maxAngle", "Maximum lock", 5, 70, 0.5, "°"),
        factor: 180 / Math.PI,
      },
    ],
  },
  {
    title: "Aero",
    fields: [
      number("aero.dragCoefficient", "Drag coefficient", 0.05, 2, 0.01, "Cd"),
      number("aero.frontalArea", "Frontal area", 0.5, 5, 0.05, "m²"),
    ],
  },
];

function getPath(target: any, path: string): any {
  let current = target;
  for (const key of path.split(".")) {
    current = current?.[key];
  }
  return current;
}

function setPath(target: any, path: string, value: any): void {
  const keys = path.split(".");
  let current = target;
  for (const key of keys.slice(0, -1)) {
    current = current[key];
  }
  current[keys[keys.length - 1]] = value;
}

function formatNumber(value: number, step = 0.01): string {
  if (!Number.isFinite(value)) return "0";
  if (step >= 1) return String(Math.round(value));
  return String(Number(value.toFixed(4)));
}

export function setupCarSetupDialog(
  viewer: SetupViewer,
  carId: string,
  trackId: string,
): void {
  const dialog = document.querySelector<HTMLDialogElement>("#car-setup-dialog");
  const openButton = document.querySelector<HTMLButtonElement>("#setup-open");
  const body = document.querySelector<HTMLElement>("#car-setup-body");
  const status = document.querySelector<HTMLElement>("#car-setup-status");
  const applyButton = document.querySelector<HTMLButtonElement>("#car-setup-apply");
  const resetButton = document.querySelector<HTMLButtonElement>("#car-setup-reset");
  const copyButton = document.querySelector<HTMLButtonElement>("#car-setup-copy");
  const closeButton = document.querySelector<HTMLButtonElement>("#car-setup-close");
  const importText = document.querySelector<HTMLTextAreaElement>("#car-setup-import-text");
  const importButton = document.querySelector<HTMLButtonElement>("#car-setup-import-apply");
  if (
    !dialog ||
    !openButton ||
    !body ||
    !status ||
    !applyButton ||
    !resetButton ||
    !copyButton ||
    !closeButton ||
    !importText ||
    !importButton
  ) {
    return;
  }

  const inputs = new Map<string, HTMLInputElement>();
  const storageKey = `lapster.setup.${carId}`;

  for (const section of SECTIONS) {
    const details = document.createElement("details");
    details.className = "setup-section";
    details.open = Boolean(section.open);
    const summary = document.createElement("summary");
    summary.textContent = section.title;
    details.append(summary);

    const grid = document.createElement("div");
    grid.className = "setup-grid";
    for (const field of section.fields) {
      const label = document.createElement("label");
      label.className =
        field.kind === "json" ? "setup-field setup-field-wide" : "setup-field";
      const name = document.createElement("span");
      name.textContent = field.label;
      const input = document.createElement("input");
      input.type = field.kind === "json" ? "text" : "number";
      input.id = `setup-${field.path.replace(/[^a-zA-Z0-9]/g, "-")}`;
      input.dataset.path = field.path;
      if (field.kind === "json") {
        input.className = "setup-json";
        input.spellcheck = false;
      } else {
        input.min = String(field.min ?? 0);
        input.max = String(field.max ?? 0);
        input.step = String(field.step ?? 0.01);
      }
      input.addEventListener("input", () => {
        status.textContent = "Unsaved changes";
        status.className = "setup-status";
      });
      label.append(name, input);
      if (field.unit) {
        const unit = document.createElement("span");
        unit.className = "setup-unit";
        unit.textContent = field.unit;
        label.append(unit);
      }
      grid.append(label);
      inputs.set(field.path, input);
    }
    details.append(grid);
    body.append(details);
  }

  const fieldForPath = (path: string) => {
    for (const section of SECTIONS) {
      const field = section.fields.find((candidate) => candidate.path === path);
      if (field) return field;
    }
    return undefined;
  };

  const populate = (setup: any) => {
    for (const [path, input] of inputs) {
      const field = fieldForPath(path);
      const value = getPath(setup, path);
      if (field?.kind === "json") {
        input.value = JSON.stringify(value);
      } else {
        const factor = field?.factor ?? 1;
        input.value = formatNumber(Number(value) * factor, field?.step);
      }
    }
  };

  const readForm = () => {
    const setup = JSON.parse(viewer.car_setup());
    for (const [path, input] of inputs) {
      const field = fieldForPath(path);
      if (field?.kind === "json") {
        const parsed = JSON.parse(input.value);
        if (!Array.isArray(parsed)) {
          throw new Error(`${field.label} must be a JSON array`);
        }
        setPath(setup, path, parsed);
      } else {
        const value = Number(input.value);
        if (!Number.isFinite(value)) {
          throw new Error(`${field?.label ?? path} is not a number`);
        }
        setPath(setup, path, value / (field?.factor ?? 1));
      }
    }
    return setup;
  };

  const persist = (setup: any) => {
    try {
      localStorage.setItem(storageKey, JSON.stringify(setup));
    } catch {
      // Storage may be unavailable in private or embedded contexts.
    }
  };

  const showStatus = (message: string, error = false) => {
    status.textContent = message;
    status.className = error ? "setup-status setup-error" : "setup-status";
  };

  const apply = () => {
    try {
      const setup = readForm();
      viewer.apply_car_setup(JSON.stringify(setup));
      const applied = JSON.parse(viewer.car_setup());
      populate(applied);
      persist(applied);
      showStatus("Settings applied");
    } catch (error) {
      showStatus(error instanceof Error ? error.message : String(error), true);
    }
  };

  const reset = () => {
    try {
      viewer.reset_car_setup();
      populate(JSON.parse(viewer.car_setup()));
      try {
        localStorage.removeItem(storageKey);
      } catch {
        // Ignore storage failures.
      }
      showStatus("Reset to car defaults");
    } catch (error) {
      showStatus(error instanceof Error ? error.message : String(error), true);
    }
  };

  const copyText = async (text: string) => {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.append(textarea);
    textarea.select();
    document.execCommand("copy");
    textarea.remove();
  };

  const copy = async () => {
    try {
      const payload = {
        schema: SETUP_SCHEMA,
        app: APP_VERSION,
        car: carId,
        track: trackId,
        setup: JSON.parse(viewer.car_setup()),
      };
      await copyText(JSON.stringify(payload, null, 2));
      showStatus("Copied. Paste it into the chat.");
    } catch (error) {
      showStatus(error instanceof Error ? error.message : String(error), true);
    }
  };

  const importSetup = () => {
    try {
      const parsed = JSON.parse(importText.value);
      const setup = parsed?.setup ?? parsed;
      if (parsed?.schema && parsed.schema !== SETUP_SCHEMA) {
        throw new Error(`Unsupported settings schema: ${parsed.schema}`);
      }
      viewer.apply_car_setup(JSON.stringify(setup));
      const applied = JSON.parse(viewer.car_setup());
      populate(applied);
      persist(applied);
      importText.value = "";
      showStatus("Pasted settings applied");
    } catch (error) {
      showStatus(error instanceof Error ? error.message : String(error), true);
    }
  };

  const stored = (() => {
    try {
      return localStorage.getItem(storageKey);
    } catch {
      return null;
    }
  })();
  if (stored) {
    try {
      viewer.apply_car_setup(stored);
    } catch {
      try {
        localStorage.removeItem(storageKey);
      } catch {
        // Ignore storage failures.
      }
    }
  }
  populate(JSON.parse(viewer.car_setup()));

  applyButton.addEventListener("click", apply);
  resetButton.addEventListener("click", reset);
  copyButton.addEventListener("click", copy);
  importButton.addEventListener("click", importSetup);
  closeButton.addEventListener("click", () => dialog.close());
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) {
      dialog.close();
    }
  });
  openButton.addEventListener("click", () => {
    populate(JSON.parse(viewer.car_setup()));
    showStatus("Changes apply when you press Apply");
    dialog.showModal();
  });
}
