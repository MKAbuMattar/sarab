function renderMonitors() {
  const box = $("#monitors");
  const ds = state.displays;
  if (!ds.length) {
    box.replaceChildren();
    return;
  }
  const minX = Math.min(...ds.map((d) => d.x)),
    minY = Math.min(...ds.map((d) => d.y));
  const maxX = Math.max(...ds.map((d) => d.x + d.width)),
    maxY = Math.max(...ds.map((d) => d.y + d.height));
  const gap = 8,
    W = box.clientWidth || 600,
    H = 170;
  const scale = Math.min(
    (W - gap * (ds.length - 1)) / (maxX - minX),
    H / (maxY - minY),
  );
  const offX = (W - (maxX - minX) * scale - gap * (ds.length - 1)) / 2;
  const order = [...ds.keys()].sort((a, b) => ds[a].x - ds[b].x);
  box.replaceChildren(
    ...ds.map((d, i) => {
      const kind = d.wallpaper ? kindOf(d.wallpaper) : null;
      const thumb =
        d.wallpaper &&
        state.library.find((w) => w.id === d.wallpaper)?.thumb_url;
      const b = el(
        "button",
        {
          type: "button",
          class: thumb ? "monitor has-thumb" : "monitor",
          "aria-pressed": String(selected === i),
          "aria-label": `${t("displays.n", { n: i + 1 })}: ${d.wallpaper ? title(d.wallpaper) : t("displays.empty")}, ${t(`reason.${d.reason}`)}`,
          onclick: () => {
            selected = selected === i ? null : i;
            render();
          },
        },
        el("span", { class: "num" }, String(i + 1)),
        thumb ? "" : icon(kind ? (ICON[kind] ?? "") : ""),
        d.wallpaper
          ? el("span", { class: `badge ${d.state}` }, t(`state.${d.state}`))
          : "",
        el(
          "span",
          { class: "label" },
          d.wallpaper ? title(d.wallpaper) : t("displays.empty"),
        ),
      );
      const slot = order.indexOf(i);
      Object.assign(b.style, {
        left: `${offX + (d.x - minX) * scale + slot * gap}px`,
        top: `${(d.y - minY) * scale}px`,
        width: `${d.width * scale}px`,
        height: `${d.height * scale}px`,
      });
      if (thumb)
        b.style.backgroundImage = `linear-gradient(rgba(0,0,0,.15), rgba(0,0,0,.6)), url(${JSON.stringify(thumb)})`;
      return b;
    }),
  );
}

function renderDetail() {
  const box = $("#detail");
  const idx = selected ?? (state.displays.length === 1 ? 0 : null);
  if (idx === null) {
    const paused = state.displays.filter(
      (d) => d.wallpaper && d.state !== "Play",
    );
    const why = paused.length
      ? t(`reason.${paused[0].reason}`)
      : t("detail.pickHint");
    box.replaceChildren(
      el("span", { class: "why caption" }, why),
      paused.length
        ? btn("", "", t("detail.playAnyway"), () =>
            run(() => invoke("play_anyway")),
          )
        : "",
      state.manual === false
        ? btn("", "", t("detail.auto"), () => run(() => invoke("resume_auto")))
        : "",
    );
    return;
  }
  const d = state.displays[idx];
  box.replaceChildren(
    el(
      "span",
      { class: "why" },
      el("strong", {}, t("displays.n", { n: idx + 1 })),
      " · ",
      d.wallpaper
        ? `${title(d.wallpaper)} · ${t(`reason.${d.reason}`)}`
        : t("displays.empty"),
      d.error
        ? el(
            "span",
            { class: "caption" },
            ` · ${t("error.prefix")}: ${d.error}`,
          )
        : "",
    ),
    d.wallpaper && d.state !== "Play"
      ? btn("", "", t("detail.playAnyway"), () =>
          run(() => invoke("play_anyway")),
        )
      : "",
    state.manual === false
      ? btn("", "", t("detail.auto"), () => run(() => invoke("resume_auto")))
      : "",
    d.wallpaper
      ? btn("", "", t("library.customize"), () => openProps(idx))
      : "",
    playlistPicker(d),
    d.wallpaper
      ? btn("subtle", "", t("displays.close"), () =>
          run(() => invoke("close", { display: idx })),
        )
      : "",
  );
}

function playlistPicker(d) {
  if (state.settings.span) return "";
  const current = state.settings.display_playlists?.[d.key] || "";
  const pick = el("select", {
    "aria-label": t("displays.playlist"),
    "data-same": "true",
  });
  playlistOptions(pick, current, true);
  pick.addEventListener("change", async () => {
    const v = await chooseSource(pick, current);
    if (v !== null) await setDisplayPlaylist(d.key, v);
  });
  return el(
    "label",
    { class: "playlist" },
    el("span", { class: "caption" }, t("displays.playlist")),
    pick,
  );
}
