function openEdit(w) {
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
