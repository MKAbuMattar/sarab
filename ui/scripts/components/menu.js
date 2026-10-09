function closeMenu() {
  const m = $("#menu");
  if (m.hidden) return;
  m.hidden = true;
  m.replaceChildren();
  menuReturn?.focus({ preventScroll: true });
}
let menuReturn = null;
function openMenu(x, y, buttons, keyboard) {
  const m = $("#menu");
  m.replaceChildren(
    ...buttons.map((b) =>
      el(
        "button",
        {
          type: "button",
          role: "menuitem",
          class: b.classList.contains("danger") ? "danger" : "",
          onclick: () => {
            closeMenu();
            b.click();
          },
        },
        icon(b.querySelector(".icon")?.textContent ?? ""),
        el(
          "span",
          {},
          b.getAttribute("aria-label") ||
            b.querySelector("span")?.textContent ||
            "",
        ),
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
document.addEventListener("contextmenu", (e) => {
  if (e.target.closest("input, textarea")) return;
  e.preventDefault();
  const tile = e.target.closest(".tile");
  const buttons = tile
    ? [...tile.querySelectorAll(".actions button, .row button")]
    : [];
  if (!buttons.length) return closeMenu();
  menuReturn = document.activeElement;
  const keyboard = !e.clientX && !e.clientY;
  const at = keyboard
    ? ((r) => [r.left, r.bottom])(
        tile.querySelector(".body").getBoundingClientRect(),
      )
    : [e.clientX, e.clientY];
  openMenu(...at, buttons, keyboard);
});
document.addEventListener("pointerdown", (e) => {
  if (!e.target.closest("#menu")) closeMenu();
});
addEventListener("blur", closeMenu);
addEventListener("resize", closeMenu);
document.addEventListener("scroll", closeMenu, true);
$("#menu").addEventListener("keydown", (e) => {
  const items = [...$("#menu").querySelectorAll("button")];
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
