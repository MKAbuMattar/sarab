function renderPresets() {
  const mb = (n) => Math.round(n / 1e6);
  $("#presets").replaceChildren(
    ...state.presets.map((p) =>
      el(
        "li",
        { class: "tile" },
        el("div", { class: "thumb" }, icon("\uE714")),
        el(
          "div",
          { class: "body" },
          el("span", { class: "name" }, p.title),
          el(
            "span",
            { class: "caption" },
            `${p.width}×${p.height} · ${p.fps} fps · ${mb(p.size)} MB`,
          ),
          el("span", { class: "caption" }, p.description),
          el("span", { class: "caption" }, p.credit),
          el(
            "div",
            { class: "row" },
            p.installed
              ? btn("accent", "\uE7F4", t("library.set"), () =>
                  run(() =>
                    invoke("set_wallpaper", {
                      target: p.id,
                      display: selectedDisplay(),
                    }),
                  ),
                )
              : p.progress != null
                ? el(
                    "span",
                    { class: "caption" },
                    t("presets.downloading", { p: p.progress }),
                  )
                : btn("", "\uE896", t("presets.get", { mb: mb(p.size) }), () =>
                    run(() => invoke("get_preset", { id: p.id })),
                  ),
          ),
        ),
      ),
    ),
  );
}
