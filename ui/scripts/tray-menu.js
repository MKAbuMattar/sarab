// Sarab's tray menu, in its own small window. Rust sizes and places the window once the
// items are drawn, and hides it when it loses focus.
const pick = (id) => invoke("tray_menu_pick", { id });

async function open() {
  const { theme, dir, items } = await invoke("tray_menu_items");
  document.documentElement.dataset.theme = theme;
  document.documentElement.dir = dir;
  const m = $("#menu");
  m.replaceChildren(
    ...items.map((it) =>
      el(
        "button",
        {
          type: "button",
          role: "menuitem",
          class: it.id === "quit" ? "sep" : "",
          onclick: () => pick(it.id),
        },
        icon(it.glyph),
        el("span", {}, it.label),
      ),
    ),
  );
  const r = m.getBoundingClientRect();
  await invoke("tray_menu_show", { width: r.width, height: r.height });
  m.focus({ preventScroll: true });
}

document.addEventListener("keydown", (e) => {
  const items = [...document.querySelectorAll("#menu button")];
  const i = items.indexOf(document.activeElement);
  const go = (n) => {
    e.preventDefault();
    items[(i + n + items.length) % items.length].focus();
  };
  if (e.key === "ArrowDown") go(1);
  else if (e.key === "ArrowUp") go(i < 0 ? 0 : -1);
  else if (e.key === "Escape") pick("");
});
document.addEventListener("contextmenu", (e) => e.preventDefault());

listen("tray-menu-open", open);
open();
