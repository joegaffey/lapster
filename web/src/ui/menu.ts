const STYLE_ID = "lapster-app-menu-style";

const STYLE = `
#viewer-controls > #menu-open {
  display: grid;
  place-items: center;
  min-width: 44px;
  min-height: 44px;
  box-sizing: border-box;
  padding: 0 10px;
  border: 1px solid #3a4350;
  border-radius: 4px;
  background: #151a21;
  color: #e6e8eb;
  font: inherit;
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
}
#viewer-controls > #menu-open:hover {
  background: #1c2430;
}
#viewer-controls > #menu-open:focus-visible {
  outline: 2px solid #38bdf8;
  outline-offset: 2px;
}
#app-menu {
  width: min(360px, calc(100vw - 24px));
  max-height: min(80vh, 640px);
  margin: auto;
  padding: 0;
  border: 1px solid #3a4350;
  border-radius: 6px;
  background: #0d1117;
  color: #e6e8eb;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.55);
  overflow: hidden;
}
#app-menu[open] {
  display: flex;
  flex-direction: column;
}
#app-menu::backdrop {
  background: rgba(0, 0, 0, 0.62);
  backdrop-filter: blur(2px);
}
#app-menu .app-menu-header {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 0;
  padding: 12px 14px;
  border-bottom: 1px solid #2a303a;
  background: #0d1117;
}
#app-menu .app-menu-header h2 {
  margin: 0;
  color: #f8fafc;
  font-size: 14px;
  letter-spacing: 0;
  text-transform: none;
}
#menu-close {
  padding: 6px 10px;
  border: 1px solid #3a4350;
  border-radius: 3px;
  background: #151a21;
  color: #e6e8eb;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
#menu-close:hover {
  background: #1c2430;
}
#app-menu .app-menu-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
  padding: 10px;
  overflow-y: auto;
  overscroll-behavior: contain;
}
#app-menu .app-menu-body > * {
  display: flex;
  box-sizing: border-box;
  align-items: center;
  width: 100%;
  min-height: 44px;
  padding: 10px 12px;
  border: 1px solid #2a303a;
  border-radius: 4px;
  background: #151a21;
  color: #e6e8eb;
  font: inherit;
  font-size: 14px;
  line-height: 1.3;
  text-align: left;
  text-decoration: none;
  white-space: normal;
  cursor: pointer;
}
#app-menu .app-menu-body > *:hover {
  background: #1c2430;
}
#app-menu .app-menu-body > *:focus-visible {
  outline: 2px solid #38bdf8;
  outline-offset: 2px;
}
#app-menu .app-menu-body > .app-menu-field {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 10px;
  cursor: default;
}
#app-menu .app-menu-body > .app-menu-field:hover {
  background: #151a21;
}
#app-menu .app-menu-field > span {
  min-width: 0;
}
#app-menu .app-menu-field select {
  min-width: 96px;
  min-height: 40px;
  padding: 4px 8px;
  border: 1px solid #3a4350;
  border-radius: 4px;
  background: #0d1117;
  color: #e6e8eb;
  font: inherit;
  font-size: 14px;
  cursor: pointer;
}
#app-menu .app-menu-field select:focus-visible {
  outline: 2px solid #38bdf8;
  outline-offset: 2px;
}
@media (pointer: coarse), (max-width: 720px) {
  #viewer-controls {
    top: 12px;
    right: 12px;
    left: auto;
    width: auto;
    max-width: none;
    justify-content: flex-end;
  }
}
`;

function ensureStyle(): void {
  if (document.querySelector(`#${STYLE_ID}`)) {
    return;
  }
  const style = document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = STYLE;
  document.head.append(style);
}

/**
 * Collapse the viewer controls into a single hamburger button that opens a
 * native menu dialog. Existing controls keep their IDs and handlers; they are
 * moved into the dialog rather than rebuilt.
 */
export function setupMenu(): void {
  const controls = document.querySelector<HTMLElement>("#viewer-controls");
  if (!controls || document.querySelector("#app-menu")) {
    return;
  }

  ensureStyle();

  const dialog = document.createElement("dialog");
  dialog.id = "app-menu";
  dialog.setAttribute("aria-label", "App menu");

  const header = document.createElement("header");
  header.className = "app-menu-header";
  const title = document.createElement("h2");
  title.textContent = "Menu";
  const closeButton = document.createElement("button");
  closeButton.id = "menu-close";
  closeButton.type = "button";
  closeButton.textContent = "Close";
  header.append(title, closeButton);

  const body = document.createElement("div");
  body.className = "app-menu-body";

  // Move the existing controls (and the Info toggle injected by setupHud)
  // into the dialog. Their listeners move with the nodes. Marked static
  // controls stay in the toolbar beside the hamburger.
  for (const child of Array.from(controls.childNodes)) {
    if (child instanceof Element && child.hasAttribute("data-menu-static")) {
      continue;
    }
    body.append(child);
  }

  dialog.append(header, body);
  document.body.append(dialog);

  const openButton = document.createElement("button");
  openButton.id = "menu-open";
  openButton.type = "button";
  openButton.textContent = "☰";
  openButton.setAttribute("aria-label", "Menu");
  openButton.setAttribute("aria-haspopup", "dialog");
  openButton.setAttribute("aria-controls", "app-menu");
  openButton.setAttribute("aria-expanded", "false");
  controls.append(openButton);

  const setExpanded = (expanded: boolean) => {
    openButton.setAttribute("aria-expanded", String(expanded));
  };

  const close = () => {
    if (dialog.open) {
      dialog.close();
    }
    setExpanded(false);
  };

  openButton.addEventListener("click", () => {
    if (dialog.open) {
      return;
    }
    dialog.showModal();
    setExpanded(true);
  });

  closeButton.addEventListener("click", close);
  dialog.addEventListener("close", () => setExpanded(false));
  dialog.addEventListener("cancel", () => setExpanded(false));

  // Close before the activated item's own handler runs so that Settings,
  // Tuning, and Info can open as a second layer. Links still navigate.
  dialog.addEventListener(
    "click",
    (event) => {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }
      if (target.closest("#menu-close")) {
        return;
      }
      if (target.closest("a, button")) {
        close();
      }
    },
    true,
  );

  // Some dialogs (for example the settings dialog) add their opener button to
  // #viewer-controls after setup. Keep the hamburger as the only visible
  // control by moving late additions into the menu as well.
  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      for (const node of Array.from(mutation.addedNodes)) {
        if (node === openButton) {
          continue;
        }
        if (node instanceof Element && node.hasAttribute("data-menu-static")) {
          continue;
        }
        body.append(node);
      }
    }
  });
  observer.observe(controls, { childList: true });
}
