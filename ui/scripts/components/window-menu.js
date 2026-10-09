// Sarab's own window menu, in place of the Windows one: right-click on the title bar,
// a click on its icon, or Alt+Space.
listen("system-menu", ({ payload: p }) => {
  const go = (action) => () => invoke("system_menu", { action });
  menuReturn = document.activeElement;
  openMenu(
    p.x / devicePixelRatio,
    p.y / devicePixelRatio,
    [
      { glyph: "", label: t("menu.restore"), disabled: !p.maximized, run: go("restore") },
      { glyph: "", label: t("menu.move"), disabled: p.maximized, run: go("move") },
      { glyph: "", label: t("menu.size"), disabled: p.maximized, run: go("size") },
      { glyph: "", label: t("menu.minimize"), run: go("minimize") },
      { glyph: "", label: t("menu.maximize"), disabled: p.maximized, run: go("maximize") },
      { glyph: "", label: t("menu.close"), keys: "Alt+F4", sep: true, run: go("close") },
    ],
    p.keyboard,
  );
});
