export function setupTouchControls(keys: Set<string>, onReset: () => void) {
  const root = document.querySelector<HTMLElement>("#touch-controls");
  if (!root) {
    return;
  }

  const counts = new Map<string, number>();
  const pointers = new Map<number, { key: string; button: HTMLButtonElement }>();

  const press = (button: HTMLButtonElement, key: string, pointerId: number) => {
    if (!key) {
      return;
    }
    pointers.set(pointerId, { key, button });
    const count = counts.get(key) ?? 0;
    counts.set(key, count + 1);
    keys.add(key);
    button.classList.add("active");
  };

  const release = (pointerId: number) => {
    const entry = pointers.get(pointerId);
    if (!entry) {
      return;
    }
    pointers.delete(pointerId);
    const count = (counts.get(entry.key) ?? 1) - 1;
    if (count <= 0) {
      counts.delete(entry.key);
      keys.delete(entry.key);
      entry.button.classList.remove("active");
    } else {
      counts.set(entry.key, count);
    }
  };

  for (const button of root.querySelectorAll<HTMLButtonElement>("[data-key]")) {
    const key = button.dataset.key ?? "";
    button.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      try {
        button.setPointerCapture(event.pointerId);
      } catch {
        // Synthetic or already-released pointers cannot be captured.
      }
      press(button, key, event.pointerId);
    });
    button.addEventListener("pointerup", (event) => {
      event.preventDefault();
      release(event.pointerId);
    });
    button.addEventListener("pointercancel", (event) => release(event.pointerId));
    button.addEventListener("lostpointercapture", (event) => release(event.pointerId));
    button.addEventListener("contextmenu", (event) => event.preventDefault());
  }

  root.querySelector<HTMLButtonElement>("#touch-reset")?.addEventListener("click", (event) => {
    event.preventDefault();
    onReset();
  });
}
