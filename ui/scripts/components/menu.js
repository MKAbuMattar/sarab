function closeMenu() {
  const m = $("#menu");
  if (m.hidden) return;
  m.hidden = true;
  m.replaceChildren();
  menuReturn?.focus({ preventScroll: true });
}
let menuReturn = null;
function openMenu(x, y, items, keyboard) {
  const m = $("#menu");
  m.replaceChildren(
    ...items.map((it) =>
      el(
        "button",
        {
          type: "button",
          role: "menuitem",
          class: [it.danger && "danger", it.sep && "sep"].filter(Boolean).join(" "),
          disabled: it.disabled || undefined,
          onclick: () => {
            closeMenu();
            it.run();
          },
        },
        icon(it.glyph),
        el("span", {}, it.label),
        it.keys ? el("span", { class: "keys" }, it.keys) : "",
      ),
    ),
  );
  m.hidden = false;
  const r = m.getBoundingClientRect();
  const left =
    document.documentElement.dir === "rtl"
      ? x - r.width < 4
        ? x
        : x - r.width
      : x + r.width > innerWidth
        ? x - r.width
        : x;
  m.style.left = `${Math.max(4, left)}px`;
  m.style.top = `${Math.max(4, y + r.height > innerHeight ? y - r.height : y)}px`;
  (keyboard ? m.querySelector("button") : m).focus({ preventScroll: true });
}
function itemsFromButtons(buttons) {
  return buttons.map((b) => ({
    glyph: b.querySelector(".icon")?.textContent ?? "",
    label:
      b.getAttribute("aria-label") ||
      b.querySelector("span")?.textContent ||
      "",
    danger: b.classList.contains("danger"),
    run: () => b.click(),
  }));
}

document.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  const field = e.target.closest(
    "input:not([type]), input[type=text], input[type=search], input[type=number], input[type=password], textarea",
  );
  const tile = e.target.closest(".tile");
  const items = field
    ? textItems(field)
    : tile
      ? itemsFromButtons([
          ...tile.querySelectorAll(".actions button, .row button"),
        ])
      : [];
  if (!items.length) return closeMenu();
  menuReturn = field || document.activeElement;
  const keyboard = !e.clientX && !e.clientY;
  const box = (field || tile.querySelector(".body")).getBoundingClientRect();
  openMenu(
    ...(keyboard ? [box.left, box.bottom] : [e.clientX, e.clientY]),
    items,
    keyboard,
  );
});
document.addEventListener("pointerdown", (e) => {
  if (!e.target.closest("#menu")) closeMenu();
});
addEventListener("blur", closeMenu);
addEventListener("resize", closeMenu);
document.addEventListener("scroll", closeMenu, true);
$("#menu").addEventListener("keydown", (e) => {
  const items = [...$("#menu").querySelectorAll("button:not(:disabled)")];
  const i = items.indexOf(document.activeElement);
  const go = (n) => {
    e.preventDefault();
    items[(i + n + items.length) % items.length].focus();
  };
  if (e.key === "ArrowDown") go(1);
  else if (e.key === "ArrowUp") go(-1);
  else if (e.key === "Home") go(-i);
  else if (e.key === "End") go(items.length - 1 - i);
  else if (e.key === "Escape" || e.key === "Tab") {
    e.preventDefault();
    closeMenu();
  }
});
