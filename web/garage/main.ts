import init, { GarageViewer } from "../pkg/sim_render.js";
import { IndexedDbPackCache } from "../src/assets/cache.ts";
import { loadPackFromUrl, withPackVersion } from "../src/assets/loader.ts";
import { setupHud } from "../src/ui/hud.ts";
import { setupMenu } from "../src/ui/menu.ts";
import { APP_VERSION } from "../src/version.generated.ts";
import { checkForUpdate } from "../src/update.ts";
import { setupPwa } from "../src/pwa.ts";
import {
  CAR_OPTIONS,
  findCar,
  loadCarPaint,
  loadSelectedCar,
  saveCarPaint,
  saveSelectedBay,
  saveSelectedCar,
  selectedBay as getSelectedBay,
} from "../src/ui/car-selection.ts";
import { loadSelectedTrack } from "../src/ui/track-selection.ts";
import { setupTrackCamera } from "../src/ui/track-camera.ts";
import { setupAssistsDialog } from "../src/ui/assists.ts";
import { recordFrame } from "../src/ui/fps.ts";
import { hideLoading, showLoadingFailure } from "../src/ui/loading.ts";
import { versionedWasmUrl } from "../src/wasm-init.ts";

declare global {
  interface Window {
    __garageViewerReady?: boolean;
    __garageViewerError?: string;
    __garageViewer?: GarageViewer;
    __garageCarsLoaded?: number;
    __garageCarsTotal?: number;
  }
}

interface GarageBayMeta {
  index: number;
  loaded: boolean;
  name: string;
  carId: string;
  color: [number, number, number];
  colorName: string;
  paint: number;
  paintName: string;
  recolorable: boolean;
}

const canvas = document.querySelector<HTMLCanvasElement>("#view")!;
let renderScale = 1;
const status = document.querySelector<HTMLElement>("#status")!;
const asset = document.querySelector<HTMLElement>("#asset")!;
const carStats = document.querySelector<HTMLElement>("#car-stats")!;
const wheels = document.querySelector<HTMLElement>("#wheels")!;
const stats = document.querySelector<HTMLElement>("#stats")!;
const surfaces = document.querySelector<HTMLElement>("#surfaces")!;
const fpsLabel = document.querySelector<HTMLElement>("#fps")!;
const frameLabel = document.querySelector<HTMLElement>("#frame-ms")!;
const steerInput = document.querySelector<HTMLInputElement>("#steer")!;
const spinInput = document.querySelector<HTMLInputElement>("#spin")!;
const steerValue = document.querySelector<HTMLElement>("#steer-value")!;
const spinValue = document.querySelector<HTMLElement>("#spin-value")!;
const animateInput = document.querySelector<HTMLInputElement>("#animate")!;
const bayLabel = document.querySelector<HTMLElement>("#bay-label")!;
const paintPanel = document.querySelector<HTMLElement>("#paint-panel")!;
const paintOptions = document.querySelector<HTMLElement>("#paint-options")!;
const carLoading = document.querySelector<HTMLElement>("#car-loading")!;
const carLoadingLabel = document.querySelector<HTMLElement>(
  "[data-car-loading-label]",
)!;
const bayPrev = document.querySelector<HTMLButtonElement>("#bay-prev")!;
const bayNext = document.querySelector<HTMLButtonElement>("#bay-next")!;
const driveCar = document.querySelector<HTMLAnchorElement>("#drive-car")!;
const carNav = document.querySelector<HTMLElement>("#car-nav")!;
const CAR_NAV_HIDE_DELAY_MS = 5000;
let carNavHideTimer = 0;

const scheduleCarNavHide = () => {
  window.clearTimeout(carNavHideTimer);
  carNavHideTimer = window.setTimeout(() => {
    if (carNav.querySelector("button:focus-visible")) {
      scheduleCarNavHide();
      return;
    }
    carNav.classList.add("car-nav-hidden");
  }, CAR_NAV_HIDE_DELAY_MS);
};

const showCarNav = () => {
  carNav.classList.remove("car-nav-hidden");
  scheduleCarNavHide();
};

window.addEventListener("pointermove", showCarNav, { passive: true });
window.addEventListener("pointerdown", showCarNav, { passive: true });
window.addEventListener("keydown", showCarNav);
carNav.addEventListener("focusin", showCarNav);
carNav.addEventListener("focusout", scheduleCarNavHide);
document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    window.clearTimeout(carNavHideTimer);
    carNav.classList.add("car-nav-hidden");
  }
});

setupHud();
setupMenu();
checkForUpdate();

const params = new URLSearchParams(window.location.search);
const defaultTrack = loadSelectedTrack();
const requestedCar = loadSelectedCar();
const CAR_URLS = CAR_OPTIONS.map((car) => withPackVersion(car.url, APP_VERSION));

const SURFACE_COLORS: Record<number, string> = {
  0: "#33a6ff",
  1: "#ff4d33",
  2: "#ffe633",
  3: "#cc4dff",
  4: "#33ffb3",
};

async function loadPack(url: string) {
  try {
    return await loadPackFromUrl(url, new IndexedDbPackCache());
  } catch {
    return await loadPackFromUrl(url);
  }
}

function resizeCanvas() {
  const ratio = Math.min(window.devicePixelRatio || 1, 2) * renderScale;
  const width = Math.max(1, Math.floor(canvas.clientWidth * ratio));
  const height = Math.max(1, Math.floor(canvas.clientHeight * ratio));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    return true;
  }
  return false;
}

function addRows(target: HTMLElement, rows: Array<[string, string]>) {
  target.replaceChildren();
  for (const [label, value] of rows) {
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.textContent = value;
    target.append(dt, dd);
  }
}

function renderCar(car: any) {
  addRows(carStats, [
    ["mass", `${car.mass.toFixed(0)} kg`],
    ["wheelbase", `${car.wheelbase.toFixed(2)} m`],
    ["track width", car.trackWidth.map((value: number) => value.toFixed(2)).join(" / ")],
    ["meshes", String(car.meshCount)],
    ["steering wheel", car.steeringWheel ? "yes" : "no"],
  ]);

  wheels.replaceChildren();
  for (const wheel of car.wheels) {
    const item = document.createElement("li");
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = `${wheel.name} ${wheel.front ? "front" : "rear"}${wheel.driven ? " drive" : ""}`;
    const detail = document.createElement("span");
    detail.className = "muted";
    detail.textContent = `r ${wheel.radius.toFixed(2)} @ ${wheel.position
      .map((value: number) => value.toFixed(2))
      .join(", ")}`;
    item.append(name, detail);
    wheels.append(item);
  }
}

function renderDebug(debug: any) {
  asset.textContent = `${debug.asset.id} + ${debug.car.id}`;
  renderCar(debug.car);

  addRows(stats, [
    ["vertices", String(debug.stats.vertices)],
    ["triangles", String(debug.stats.triangles)],
    ["track tris", String(debug.stats.trackTriangles)],
    ["car tris", String(debug.stats.carTriangles)],
    ["draw calls", String(debug.stats.drawCalls)],
    ["collision tris", String(debug.stats.collisionTriangles)],
  ]);

  surfaces.replaceChildren();
  for (const surface of debug.surfaces ?? []) {
    const item = document.createElement("li");
    const swatch = document.createElement("span");
    swatch.className = "name";
    swatch.style.color = SURFACE_COLORS[surface.id] ?? "#e5e7eb";
    swatch.textContent = `${surface.id}: ${surface.name}`;
    const detail = document.createElement("span");
    detail.className = "muted";
    detail.textContent = `μ ${surface.friction.toFixed(2)}`;
    item.append(swatch, detail);
    surfaces.append(item);
  }
}

async function boot() {
  status.textContent = "starting";
  if (await setupPwa()) {
    return;
  }
  status.textContent = "loading wasm";
  await init(versionedWasmUrl("../pkg/sim_render_bg.wasm"));

  const carIds = CAR_OPTIONS.map((car) => car.id);
  const savedBay = getSelectedBay();
  const requestedIndex = carIds.indexOf(requestedCar.id);
  const initialBay =
    carIds[savedBay] === requestedCar.id ? savedBay : Math.max(0, requestedIndex);

  status.textContent = "loading packs";
  const initialPack = await loadPack(CAR_URLS[initialBay]);
  resizeCanvas();
  status.textContent = "creating renderer";
  const packs: Array<Uint8Array | null> = carIds.map(() => null);
  packs[initialBay] = initialPack.bytes;
  const paintable = CAR_OPTIONS.map((car) => car.paintable === true);
  const viewer = await GarageViewer.create_concrete(canvas, carIds, packs, paintable);
  window.__garageViewer = viewer;

  const loadedBays = new Set<number>([initialBay]);
  const failedBays = new Map<number, unknown>();
  const inFlight = new Map<number, Promise<void>>();
  window.__garageCarsTotal = carIds.length;
  window.__garageCarsLoaded = loadedBays.size;

  setupAssistsDialog(viewer, {
    gameplay: false,
    timeOfDay: false,
    onRenderScale: (scale) => {
      renderScale = scale;
      if (resizeCanvas()) {
        viewer.resize(canvas.width, canvas.height);
      }
    },
  });

  const debug = JSON.parse(viewer.debug_info());
  renderDebug(debug);
  const orbitCamera = setupTrackCamera(canvas, viewer, { allowPan: false });
  const bays = JSON.parse(viewer.bay_list()) as GarageBayMeta[];
  const palette = JSON.parse(viewer.paint_options()) as Array<{
    index: number;
    name: string;
    color: [number, number, number];
  }>;
  let selectedBay = initialBay;

  const carName = (id: string) => findCar(id)?.name ?? id;

  const setPaintPanelVisible = (visible: boolean) => {
    paintPanel.classList.toggle("paint-panel-visible", visible);
    if (visible) {
      paintPanel.removeAttribute("aria-hidden");
      paintPanel.removeAttribute("inert");
    } else {
      paintPanel.setAttribute("aria-hidden", "true");
      paintPanel.setAttribute("inert", "");
    }
  };

  const applySavedPaint = (index: number) => {
    const car = findCar(bays[index]?.carId ?? carIds[index]);
    const savedPaint = car ? loadCarPaint(car.id) : null;
    const paint =
      car?.paintable === true ? savedPaint ?? car.defaultPaint ?? null : savedPaint;
    if (paint === null) {
      return;
    }
    const bay = (JSON.parse(viewer.bay_list()) as GarageBayMeta[])[index];
    if (bay?.recolorable) {
      viewer.set_bay_paint(index, paint);
    }
  };
  applySavedPaint(initialBay);

  let steer = 0.3;
  let spin = 0;
  let animating = false;
  steerInput.value = String(steer);

  const applyAngles = () => {
    steerValue.textContent = steer.toFixed(2);
    spinValue.textContent = spin.toFixed(2);
    viewer.set_wheel_angles(steer, spin);
  };

  steerInput.addEventListener("input", () => {
    steer = Number(steerInput.value);
    applyAngles();
  });
  spinInput.addEventListener("input", () => {
    spin = Number(spinInput.value);
    applyAngles();
  });
  animateInput.addEventListener("change", () => {
    animating = animateInput.checked;
  });

  const swatches: HTMLButtonElement[] = [];
  for (const option of palette) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "paint-swatch";
    button.setAttribute("role", "radio");
    button.setAttribute("aria-checked", "false");
    button.setAttribute("aria-label", option.name);
    button.title = option.name;
    button.dataset.paint = String(option.index);
    const [red, green, blue] = option.color;
    button.style.setProperty(
      "--swatch",
      `rgb(${Math.round(red * 255)} ${Math.round(green * 255)} ${Math.round(blue * 255)})`,
    );
    button.addEventListener("click", () => {
      const info = JSON.parse(viewer.set_selected_paint(option.index));
      saveCarPaint(info.carId, option.index);
      applySelectedBay(info);
    });
    paintOptions.append(button);
    swatches.push(button);
  }

  const renderPaintPicker = (info: any) => {
    const recolorable = info.loaded === true && info.recolorable === true;
    setPaintPanelVisible(recolorable);
    for (const swatch of swatches) {
      const index = Number(swatch.dataset.paint);
      swatch.setAttribute(
        "aria-checked",
        String(recolorable && index === info.paint),
      );
    }
  };

  const showCarLoading = (text: string, state: "loading" | "error" = "loading") => {
    carLoading.dataset.state = state;
    carLoadingLabel.textContent = text;
    carLoading.classList.add("car-loading-visible");
    carLoading.removeAttribute("aria-hidden");
  };

  const hideCarLoading = () => {
    carLoading.classList.remove("car-loading-visible");
    carLoading.setAttribute("aria-hidden", "true");
    carLoadingLabel.textContent = "";
  };

  const updateCarLoading = () => {
    let info: any = null;
    try {
      info = JSON.parse(viewer.bay_info());
    } catch {
      info = null;
    }
    if (info && info.loaded === false) {
      if (failedBays.has(info.selected)) {
        showCarLoading(
          `Couldn't load ${carName(info.carId)} · reselect to retry`,
          "error",
        );
      } else {
        showCarLoading(`Loading ${carName(info.carId)}…`);
      }
      return;
    }
    hideCarLoading();
  };

  const frameYield = () =>
    new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

  const ensureBay = async (index: number): Promise<void> => {
    if (loadedBays.has(index)) {
      return;
    }
    const existing = inFlight.get(index);
    if (existing) {
      return existing;
    }
    failedBays.delete(index);
    const task = (async () => {
      try {
        const pack = await loadPack(CAR_URLS[index]);
        await frameYield();
        viewer.load_bay(index, pack.bytes, CAR_OPTIONS[index].paintable === true);
        loadedBays.add(index);
        window.__garageCarsLoaded = loadedBays.size;
        applySavedPaint(index);
        const info = JSON.parse(viewer.bay_info());
        if (info.selected === index) {
          applySelectedBay(info);
        }
      } catch (error) {
        failedBays.set(index, error);
        console.error(`garage could not load ${carIds[index]}`, error);
      } finally {
        inFlight.delete(index);
        updateCarLoading();
      }
    })();
    inFlight.set(index, task);
    updateCarLoading();
    return task;
  };

  const prefetchNeighbors = (index: number) => {
    const count = Math.max(1, JSON.parse(viewer.bay_info()).count);
    for (const neighbor of [index - 1, index + 1]) {
      if (
        neighbor >= 0 &&
        neighbor < count &&
        !failedBays.has(neighbor)
      ) {
        void ensureBay(neighbor);
      }
    }
  };

  const applySelectedBay = (info: any) => {
    selectedBay = info.selected;
    const car = findCar(info.carId) ?? CAR_OPTIONS[0];
    saveSelectedBay(selectedBay);
    saveSelectedCar(car.id);
    asset.textContent = `${debug.asset.id} + ${car.id}`;
    if (info.car) {
      renderCar(info.car);
    }
    const paintName = info.loaded
      ? info.paintName || "Factory livery"
      : "Loading…";
    bayLabel.textContent = `${info.name} (${selectedBay + 1}/${info.count}) · ${car.name} · ${paintName}`;
    driveCar.title = info.loaded
      ? `Drive the ${paintName} ${car.name}`
      : `Drive the ${car.name}`;
    const driveUrl = new URL("../drive/", window.location.href);
    const driveParams = new URLSearchParams(params);
    if (!driveParams.has("track")) {
      driveParams.set("track", `${defaultTrack.url}?v=${encodeURIComponent(APP_VERSION)}`);
    }
    driveParams.set("car", car.url);
    driveParams.set("bay", String(selectedBay + 1));
    if (info.loaded && info.recolorable === true) {
      driveParams.set("paint", String(info.paint));
    } else if (!info.loaded) {
      const storedPaint = loadCarPaint(car.id);
      if (storedPaint !== null) {
        driveParams.set("paint", String(storedPaint));
      } else {
        driveParams.delete("paint");
      }
    } else {
      driveParams.delete("paint");
    }
    driveUrl.search = driveParams.toString();
    driveCar.href = driveUrl.href;
    renderPaintPicker(info);
    applyAngles();
    if (info.loaded === false) {
      void ensureBay(info.selected);
      updateCarLoading();
    } else {
      updateCarLoading();
      prefetchNeighbors(info.selected);
    }
  };

  const selectBay = (index: number, animate = true) => {
    const current = JSON.parse(viewer.bay_info());
    const count = Math.max(1, current.count);
    const next = ((index % count) + count) % count;
    const cameraInfo = JSON.parse(viewer.select_bay(next));
    orbitCamera?.setHome(cameraInfo, animate);
    applySelectedBay(JSON.parse(viewer.bay_info()));
  };

  const bayOrder = (count: number): number[] =>
    Array.from({ length: count }, (_, index) => index);

  const stepBay = (delta: number) => {
    const info = JSON.parse(viewer.bay_info());
    const order = bayOrder(Math.max(1, info.count));
    const position = Math.max(0, order.indexOf(selectedBay));
    const next = order[(position + delta + order.length) % order.length];
    selectBay(next);
  };

  selectBay(initialBay, false);
  bayPrev.addEventListener("click", () => {
    stepBay(-1);
    showCarNav();
  });
  bayNext.addEventListener("click", () => {
    stepBay(1);
    showCarNav();
  });
  window.addEventListener("keydown", (event) => {
    const target = event.target as HTMLElement | null;
    if (target && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName)) {
      return;
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      stepBay(-1);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      stepBay(1);
    }
  });

  const observer = new ResizeObserver(() => {
    if (resizeCanvas()) {
      viewer.resize(canvas.width, canvas.height);
    }
  });
  observer.observe(canvas);

  let last = performance.now();
  let frames = 0;
  let accumulated = 0;
  let frameTime = 0;

  const frame = (now: number) => {
    const delta = now - last;
    last = now;
    frames += 1;
    accumulated += delta;
    frameTime += delta;
    recordFrame(delta);

    if (animating) {
      spin = (spin + (delta / 1000) * 4.0) % (Math.PI * 2);
      spinInput.value = String(spin);
      applyAngles();
    }

    if (accumulated >= 500) {
      fpsLabel.textContent = ((frames * 1000) / accumulated).toFixed(1);
      frameLabel.textContent = `${(frameTime / frames).toFixed(2)} ms`;
      frames = 0;
      accumulated = 0;
      frameTime = 0;
    }

    try {
      viewer.render();
      requestAnimationFrame(frame);
    } catch (error) {
      fail(error);
    }
  };

  status.textContent = "ready";
  status.className = "status ready";
  window.__garageViewerReady = true;
  requestAnimationFrame(frame);
  requestAnimationFrame(hideLoading);
}

function fail(error: unknown) {
  showLoadingFailure(error);
  const message = error instanceof Error ? error.message : String(error);
  window.__garageViewerError = message;
  status.textContent = message;
  status.className = "status error";
  console.error(error);
}

boot().catch(fail);
