function renderLibrary() {
  const lib = state.library;
  const kinds = [...new Set(lib.map((w) => w.kind))];
  if (filters.type && filters.type !== "all" && !kinds.includes(filters.type))
    filters.type = "all";
  const chip = (value, label) =>
    el(
      "button",
      {
        type: "button",
        "aria-pressed": String((filters.type || "all") === value),
        onclick: () => {
          filters.type = value;
          saveFilters();
          renderLibrary();
        },
      },
      label,
    );
  $("#lib-types").replaceChildren(
    chip("all", t("filter.all")),
    ...kinds.map((k) => chip(k, t(`type.${k}`))),
  );

  const cat = $("#lib-category");
  const used = [...new Set(lib.map((w) => w.info.category || "none"))];
  cat.replaceChildren(
    el("option", { value: "all" }, t("library.allCategories")),
    ...state.categories
      .filter((c) => used.includes(c))
      .map((c) => el("option", { value: c }, categoryName(c))),
    ...(used.includes("none")
      ? [el("option", { value: "none" }, t("edit.none"))]
      : []),
  );
  cat.value = [...cat.options].some((o) => o.value === filters.category)
    ? filters.category
    : "all";
  $("#lib-sort").value = filters.sort || "name";
  if (document.activeElement !== $("#lib-search"))
    $("#lib-search").value = filters.query || "";

  const shown = filterLibrary(lib, {
    type: filters.type || "all",
    category: cat.value,
    query: filters.query || "",
    sort: filters.sort || "name",
    label: categoryName,
    lang: lang(),
  });
  $("#lib-empty").hidden = lib.length > 0;
  $("#lib-none").hidden = lib.length === 0 || shown.length > 0;
  $(".lib-bar").hidden = $("#lib-types").hidden = lib.length === 0;
  $("#library").replaceChildren(
    ...shown.map((w) =>
      el(
        "li",
        { class: "tile" },
        thumbFor(w),
        el(
          "div",
          { class: "body" },
          el("span", { class: "name", title: wpTitle(w) }, wpTitle(w)),
          el(
            "span",
            { class: "caption line" },
            [
              t(`type.${w.kind}`),
              w.info.category && categoryName(w.info.category),
              w.preset && t("presets.builtIn"),
            ]
              .filter(Boolean)
              .join(" · "),
          ),
          w.too_new
            ? el(
                "span",
                { class: "caption danger-text" },
                t("library.tooNew", { v: w.info.app_version }),
              )
            : "",
          el(
            "div",
            { class: "actions" },
            btn("accent", "", t("library.set"), async () => {
              if (
                w.too_new &&
                !(await ask({
                  title: t("library.tooNewTitle"),
                  body: t("library.tooNew", { v: w.info.app_version }),
                  ok: t("library.set"),
                  cancel: t("dialog.cancel"),
                }))
              )
                return;
              if (
                w.kind === "app" &&
                !(await ask({
                  title: t("library.appTitle", { title: wpTitle(w) }),
                  body: t("library.appBody"),
                  ok: t("library.run"),
                  cancel: t("dialog.cancel"),
                  danger: true,
                }))
              )
                return;
              run(() =>
                invoke("set_wallpaper", {
                  target: w.id,
                  display: selectedDisplay(),
                }),
              );
            }),
            el("span", { class: "spacer" }),
            iconBtn("", t("library.info"), () => openInfo(w)),
            w.preset ? "" : iconBtn("", t("library.edit"), () => openEdit(w)),
            w.preset
              ? ""
              : iconBtn(
                  "",
                  t("library.delete"),
                  async () => {
                    const ok = await ask({
                      title: t("library.deleteTitle", { title: wpTitle(w) }),
                      body: t("library.deleteBody"),
                      ok: t("library.delete"),
                      cancel: t("dialog.cancel"),
                      danger: true,
                    });
                    if (ok) run(() => invoke("remove", { id: w.id }));
                  },
                  "danger",
                ),
          ),
        ),
      ),
    ),
  );
  document.querySelectorAll(".lib-bar select").forEach(combo);
}
