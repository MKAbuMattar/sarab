$("#add").addEventListener("submit", (e) => {
  e.preventDefault();
  const target = $("#target").value.trim();
  run(async () => {
    await invoke("set_wallpaper", { target, display: selectedDisplay() });
    $("#target").value = "";
  });
});
$("#lib-search").addEventListener("input", (e) => {
  filters.query = e.target.value;
  saveFilters();
  renderLibrary();
});
$("#lib-category").addEventListener("change", (e) => {
  filters.category = e.target.value;
  saveFilters();
  renderLibrary();
});
$("#lib-sort").addEventListener("change", (e) => {
  filters.sort = e.target.value;
  saveFilters();
  renderLibrary();
});
$("#check-updates").addEventListener("change", () =>
  run(() => invoke("save_settings", { new: readSettings() })),
);
$("#move-library").addEventListener("click", () =>
  run(() => invoke("move_library")),
);
$("#export-logs").addEventListener("click", () =>
  run(() => invoke("export_logs")),
);
$("#reset-settings").addEventListener("click", async () => {
  const ok = await ask({
    title: t("about.resetTitle"),
    body: t("about.resetBody"),
    ok: t("about.resetButton"),
    cancel: t("dialog.cancel"),
    danger: true,
  });
  if (!ok) return;
  await run(() => invoke("reset_settings"));
  await loadLanguage(state.settings.language || "en");
  render();
});
$("#check-now").addEventListener("click", () =>
  run(() => invoke("check_update")),
);
$("#pause").addEventListener("click", () => run(() => invoke("toggle_pause")));
document
  .querySelectorAll("[data-open]")
  .forEach((b) =>
    b.addEventListener("click", () =>
      run(() => invoke("open", { which: b.dataset.open })),
    ),
  );
$("#all-displays").addEventListener("click", () => {
  selected = null;
  render();
});
window.addEventListener("resize", () => state && renderMonitors());
document
  .querySelectorAll(".nav-item[data-page]")
  .forEach((b) => b.addEventListener("click", () => showPage(b.dataset.page)));
$("#settings").addEventListener("change", async (e) => {
  if (e.target.id === "autostart")
    return run(() => invoke("autostart", { enable: e.target.checked }));
  if (e.target.name === "language") await loadLanguage(e.target.value);
  await run(() => invoke("save_settings", { new: readSettings() }));
});

listen("changed", refresh);

(async () => {
  state = await invoke("state");
  document.documentElement.dataset.theme = state.theme;
  await loadLanguage(state.settings.language || "en");
  let page = "library";
  try {
    page = localStorage.getItem("page") || page;
  } catch {}
  showPage(["library", "settings", "about"].includes(page) ? page : "library");
  render();
})();
