let trim = null;

async function openEdit(w) {
  const d = $("#edit"),
    f = $("#edit-form");
  f.title.value = w.info.title || "";
  f.description.value = w.info.description || "";
  f.author.value = w.info.author || "";
  f.category.replaceChildren(
    el("option", { value: "" }, t("edit.none")),
    ...state.categories.map((c) => el("option", { value: c }, categoryName(c))),
  );
  f.category.value = w.info.category || "";
  f.tags.value = (w.info.tags || []).join(", ");
  $("#edit-error").hidden = true;
  combo(f.category);
  // Videos get the part-that-plays editor.
  trim?.stop();
  trim = null;
  $("#trim-slot").replaceChildren();
  d.classList.toggle("wide", w.info.type === "video");
  if (w.info.type === "video") {
    const file = await invoke("video_file", { id: w.id }).catch(() => null);
    if (file) {
      trim = trimEditor(
        window.__TAURI__.core.convertFileSrc(file),
        w.info.clip,
      );
      $("#trim-slot").append(trim.el);
    }
  }
  d.onclose = () => {
    trim?.stop();
    trim = null;
  };
  d.returnValue = "cancel";
  d.showModal();
  f.title.focus();
  f.onsubmit = async (e) => {
    if (e.submitter?.value !== "save") return;
    e.preventDefault();
    try {
      await invoke("edit_info", {
        id: w.id,
        edit: {
          title: f.title.value,
          description: f.description.value,
          author: f.author.value,
          category: f.category.value || null,
          tags: f.tags.value
            .split(",")
            .map((x) => x.trim())
            .filter(Boolean),
          clip: trim?.value(),
        },
      });
      d.close("save");
      await refresh();
    } catch (err) {
      $("#edit-error").textContent = String(err);
      $("#edit-error").hidden = false;
    }
  };
}
