const LOADING_ID = "viewer-loading";

function loadingElement(): HTMLElement | null {
  return document.querySelector<HTMLElement>(`#${LOADING_ID}`);
}

export function hideLoading(): void {
  const element = loadingElement();
  if (element) {
    element.hidden = true;
  }
}

export function showLoadingFailure(error: unknown): void {
  const element = loadingElement();
  if (!element) {
    return;
  }
  const message = error instanceof Error ? error.message : String(error);
  element.hidden = false;
  element.dataset.state = "error";
  const label = element.querySelector<HTMLElement>("[data-loading-label]");
  if (label) {
    label.textContent = message ? `Failed to load: ${message}` : "Failed to load";
  }
}
