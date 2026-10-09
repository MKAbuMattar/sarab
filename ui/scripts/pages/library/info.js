async function openInfo(w) {
  let det = {};
  try {
    det = await invoke("details", { id: w.id });
  } catch (e) {
    showError(String(e));
    return;
  }
  const mb = (n) =>
    n >= 1e6
      ? `${(n / 1e6).toFixed(1)} MB`
      : `${Math.max(1, Math.round(n / 1e3))} KB`;
  const date = (s) =>
    s
      ? new Date(s * 1000).toLocaleString(document.documentElement.lang)
      : t("info.unknown");
  const rows = [
    ["info.type", t(`type.${w.kind}`)],
    [
      "edit.category",
      w.info.category ? categoryName(w.info.category) : t("edit.none"),
    ],
    ["edit.tags", (w.info.tags || []).join(", ") || t("info.unknown")],
    ["edit.author", w.info.author || t("info.unknown")],
    ["info.license", w.info.license || t("info.unknown")],
    ["info.source", det.source || t("info.unknown")],
    [
      "info.size",
      `${mb(det.size || 0)} · ${t("info.files", { n: det.files || 0 })}`,
    ],
    ["info.added", date(det.created)],
    ["info.modified", date(det.modified)],
    ["info.version", String(w.info.version || 1)],
    ["info.customize", det.has_props ? t("info.yes") : t("info.no")],
  ];
  $("#info-title").textContent = wpTitle(w);
  $("#info-desc").textContent = shownDescription(w, lang());
  $("#info-desc").hidden = !shownDescription(w, lang());
  $("#info-list").replaceChildren(
    ...rows.flatMap(([k, v]) => [el("dt", {}, t(k)), el("dd", {}, v)]),
  );
  $("#info-folder").onclick = () => run(() => invoke("reveal", { id: w.id }));
  $("#info-export").onclick = () =>
    run(() => invoke("export_wallpaper", { id: w.id }));
  $("#info").showModal();
}
