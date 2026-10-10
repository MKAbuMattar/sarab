const PICK_FOLDER = "folder:?";

function folderName(path) {
  return path.split(/[\\/]/).filter(Boolean).pop() || path;
}

function playlistOptions(select, value, same) {
  const tags = [
    ...new Set(state.library.flatMap((w) => w.info.tags || [])),
  ].sort((a, b) => a.localeCompare(b));
  const options = [
    same ? el("option", { value: "" }, t("playlist.same")) : "",
    el("option", { value: "all" }, t("library.allCategories")),
    ...state.categories.map((c) => el("option", { value: c }, categoryName(c))),
    ...tags.map((tag) =>
      el("option", { value: `tag:${tag}` }, t("playlist.tag", { name: tag })),
    ),
    value.startsWith("folder:") && value !== PICK_FOLDER
      ? el(
          "option",
          { value },
          t("playlist.folder", { name: folderName(value.slice(7)) }),
        )
      : "",
    state.pick_folder
      ? el("option", { value: PICK_FOLDER }, t("playlist.pickFolder"))
      : "",
  ].filter(Boolean);
  select.replaceChildren(...options);
  select.value = value;
  if (select.selectedIndex < 0) select.value = same ? "" : "all";
}

async function chooseSource(select, previous) {
  if (select.value !== PICK_FOLDER) return select.value;
  const path = await invoke("pick_playlist_folder");
  const value = path ? `folder:${path}` : previous;
  playlistOptions(select, value, select.dataset.same === "true");
  select._combo?.sync();
  return path ? value : null;
}

function renderSchedule() {
  const box = $("#schedule");
  if (box.contains(document.activeElement)) return;
  const walls = state.library.filter((w) => w.kind !== "app");
  box.replaceChildren(
    ...(state.settings.schedule || []).map((slot) => {
      const pick = el(
        "select",
        { class: "slot-wallpaper", "aria-label": t("schedule.wallpaper") },
        ...walls.map((w) => el("option", { value: w.id }, wpTitle(w))),
      );
      pick.value = slot.wallpaper;
      const row = el(
        "div",
        { class: "slot" },
        el("input", {
          type: "time",
          class: "slot-at",
          value: slot.at,
          required: "",
          "aria-label": t("schedule.at"),
        }),
        pick,
        iconBtn("", t("schedule.remove"), () => {
          row.remove();
          $("#settings").dispatchEvent(new Event("change"));
        }),
      );
      return row;
    }),
  );
  $("#add-slot").disabled = !walls.length;
}

function readSchedule() {
  return [...document.querySelectorAll("#schedule .slot")]
    .map((row) => ({
      at: row.querySelector(".slot-at").value,
      wallpaper: row.querySelector(".slot-wallpaper").value,
    }))
    .filter((s) => s.at && s.wallpaper);
}

$("#add-slot").addEventListener("click", () => {
  const first = state.library.find((w) => w.kind !== "app");
  if (!first) return;
  const schedule = [
    ...readSchedule(),
    {
      at: (state.settings.schedule || []).length ? "19:00" : "07:00",
      wallpaper: first.id,
    },
  ];
  run(() => invoke("save_settings", { new: { ...readSettings(), schedule } }));
});

async function setDisplayPlaylist(key, value) {
  const display_playlists = { ...(state.settings.display_playlists || {}) };
  if (value) display_playlists[key] = value;
  else delete display_playlists[key];
  await run(() =>
    invoke("save_settings", {
      new: { ...state.settings, display_playlists },
    }),
  );
}
