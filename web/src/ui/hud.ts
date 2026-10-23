const MOBILE_QUERY = "(max-width: 720px), (pointer: coarse)";

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
  hud.prepend(toggle);

  const setCollapsed = (collapsed: boolean) => {
    hud.classList.toggle("hud-collapsed", collapsed);
    toggle.textContent = collapsed ? "Info" : "Hide info";
    toggle.setAttribute("aria-expanded", String(!collapsed));
  };

  setCollapsed(window.matchMedia(MOBILE_QUERY).matches);
  toggle.addEventListener("click", () => {
    setCollapsed(!hud.classList.contains("hud-collapsed"));
  });
}
