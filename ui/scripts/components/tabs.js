function tabs(list) {
  const shown = () => [...list.querySelectorAll('[role="tab"]')].filter((b) => !b.hidden);
  function select(tab, focus = false) {
    for (const b of list.querySelectorAll('[role="tab"]')) {
      const on = b === tab;
      b.setAttribute("aria-selected", String(on));
      b.tabIndex = on ? 0 : -1;
      document.getElementById(b.getAttribute("aria-controls")).hidden = !on;
    }
    if (focus) tab.focus();
  }
  list.addEventListener("click", (e) => {
    const b = e.target.closest('[role="tab"]');
    if (b) select(b);
  });
  list.addEventListener("keydown", (e) => {
    const t = shown();
    const i = t.indexOf(document.activeElement);
    if (i < 0) return;
    const rtl = getComputedStyle(list).direction === "rtl";
    const step = { ArrowRight: rtl ? -1 : 1, ArrowLeft: rtl ? 1 : -1 }[e.key];
    const to =
      step !== undefined
        ? t[(i + step + t.length) % t.length]
        : e.key === "Home"
          ? t[0]
          : e.key === "End"
            ? t[t.length - 1]
            : null;
    if (!to) return;
    e.preventDefault();
    select(to, true);
  });
  return {
    select: (tab, focus) => select(tab, focus),
    of: (node) => list.querySelector(`[aria-controls="${node.closest('[role="tabpanel"]')?.id}"]`),
  };
}
