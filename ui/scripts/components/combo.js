function combo(sel) {
  if (sel._combo) return sel._combo.sync();
  sel.classList.add("native");
  const text = el("span", { class: "combo-text" });
  const button = el(
    "button",
    {
      type: "button",
      class: "combo",
      "aria-haspopup": "listbox",
      "aria-expanded": "false",
    },
    text,
    icon("\uE70D"),
  );
  sel.after(button);
  const sync = () => {
    text.textContent = sel.selectedOptions[0]?.textContent ?? "";
  };
  const choose = (i) => {
    if (i < 0 || i >= sel.options.length || i === sel.selectedIndex) return;
    sel.selectedIndex = i;
    sync();
    sel.dispatchEvent(new Event("change", { bubbles: true }));
  };
  button.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      choose(sel.selectedIndex + (e.key === "ArrowDown" ? 1 : -1));
    }
  });
  button.addEventListener("click", () => {
    const host = sel.closest("dialog") ?? document.body;
    const list = el("div", { class: "flyout", role: "listbox" });
    const items = [...sel.options].map((o, i) => {
      const it = el(
        "div",
        {
          class: "option",
          role: "option",
          tabindex: "-1",
          "aria-selected": String(i === sel.selectedIndex),
        },
        o.textContent,
      );
      it.addEventListener("click", () => {
        choose(i);
        close();
      });
      return it;
    });
    list.append(...items);
    host.append(list);
    const r = button.getBoundingClientRect();
    list.style.minWidth = `${r.width}px`;
    const h = list.offsetHeight,
      w = list.offsetWidth;
    list.style.top = `${r.bottom + 4 + h > innerHeight - 8 ? Math.max(8, r.top - h - 4) : r.bottom + 4}px`;
    list.style.left = `${document.documentElement.dir === "rtl" ? r.right - w : r.left}px`;
    button.setAttribute("aria-expanded", "true");
    let cur = Math.max(0, sel.selectedIndex);
    items[cur]?.focus();
    const outside = (e) => {
      if (!list.contains(e.target) && !button.contains(e.target)) close();
    };
    function close() {
      list.remove();
      button.setAttribute("aria-expanded", "false");
      document.removeEventListener("pointerdown", outside, true);
      button.focus();
    }
    document.addEventListener("pointerdown", outside, true);
    list.addEventListener("keydown", (e) => {
      if (e.key === "Escape" || e.key === "Tab") close();
      else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        cur = Math.max(
          0,
          Math.min(items.length - 1, cur + (e.key === "ArrowDown" ? 1 : -1)),
        );
        items[cur].focus();
      } else if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        choose(cur);
        close();
      }
    });
  });
  sel._combo = { sync };
  sync();
  return sel._combo;
}
