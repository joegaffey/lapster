export function setupHud() {
  const hud = document.querySelector<HTMLElement>("#hud");
  if (!hud || hud.querySelector("#hud-toggle")) {
    return;
  }

  const toggle = document.createElement("button");
  toggle.id = "hud-toggle";
  toggle.className = "hud-toggle";
  toggle.type = "button";
  toggle.setAttribute("aria-controls", "hud");
  const host = document.querySelector<HTMLElement>("#viewer-controls") ?? hud;
  host.prepend(toggle);

  const setCollapsed = (collapsed: boolean) => {
    hud.classList.toggle("hud-collapsed", collapsed);
    toggle.textContent = collapsed ? "Info" : "Hide info";
    toggle.setAttribute("aria-expanded", String(!collapsed));
  };

  setCollapsed(true);
  toggle.addEventListener("click", () => {
    setCollapsed(!hud.classList.contains("hud-collapsed"));
  });
}
