const STYLE_ID = "lapster-fps-style";
const COUNTER_ID = "fps-counter";

let counter: HTMLElement | null = null;
let enabled = false;
let frames = 0;
let elapsedMs = 0;

const STYLE = `
#fps-counter {
  position: fixed;
  top: 16px;
  left: 16px;
  z-index: 9;
  padding: 4px 7px;
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: rgba(10, 13, 17, 0.82);
  color: #cbd5e1;
  font: 11px/1.2 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
  pointer-events: none;
}
#fps-counter[hidden] { display: none; }
#hud { z-index: 10; }
@media (max-width: 720px) {
  #fps-counter { top: 84px; left: 12px; }
}
`;

function ensureCounter(): HTMLElement {
  if (counter) {
    return counter;
  }
  if (!document.querySelector(`#${STYLE_ID}`)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.append(style);
  }
  counter = document.createElement("div");
  counter.id = COUNTER_ID;
  counter.dataset.testid = COUNTER_ID;
  counter.setAttribute("role", "status");
  counter.setAttribute("aria-label", "Frames per second");
  counter.textContent = "— fps";
  counter.hidden = true;
  document.body.append(counter);
  return counter;
}

export function setFpsCounterVisible(visible: boolean): void {
  const element = ensureCounter();
  enabled = visible;
  frames = 0;
  elapsedMs = 0;
  element.textContent = "— fps";
  element.hidden = !visible;
}

export function recordFrame(deltaMs: number): void {
  if (!enabled) {
    return;
  }
  const element = ensureCounter();
  frames += 1;
  elapsedMs += Math.max(deltaMs, 0);
  if (elapsedMs >= 500) {
    element.textContent = `${Math.round((frames * 1000) / elapsedMs)} fps`;
    frames = 0;
    elapsedMs = 0;
  }
}
