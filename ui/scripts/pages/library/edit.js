let trim = null;
let thumbs = null;
let editTabs = null;

const editField = (message) => {
  const m = String(message);
  if (/part that plays|start of 0/i.test(m)) return "#trim [name=start]";
  if (/image|thumbnail/i.test(m)) return "#edit-panel-thumb [value=image]";
  if (/tag/i.test(m)) return "#edit-form [name=tags]";
  if (/categor/i.test(m)) return "#edit-form [name=category]";
  if (/description|author/i.test(m)) return "#edit-form [name=description]";
  return "#edit-form [name=title]";
};

function showEditField(node) {
  if (!node) return;
  editTabs.select(editTabs.of(node));
  node.focus();
}

async function openEdit(w) {
  const d = $("#edit"),
    f = $("#edit-form");
  if (!editTabs) {
    editTabs = tabs($("#edit .tabs"));
    f.addEventListener(
      "invalid",
      (e) => {
        if (f.querySelector(":invalid") === e.target) showEditField(e.target);
      },
      { capture: true },
    );
  }
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
  trim?.stop();
  trim = null;
  $("#trim-slot").replaceChildren();
  const video = w.info.type === "video";
  d.classList.toggle("wide", video);
  $("#edit-tab-part").hidden = !video;
  $("#edit-tab-thumb").hidden = w.info.type === "picture";
  thumbs = thumbChoice(w);
  $("#edit-panel-thumb").replaceChildren(thumbs.el);
  d.querySelectorAll(".tabs .dirty").forEach((x) => (x.hidden = true));
  editTabs.select($("#edit-tab-details"));
  if (video) {
    const file = await invoke("video_file", { id: w.id }).catch(() => null);
    if (file) {
      trim = trimEditor(
        window.__TAURI__.core.convertFileSrc(file),
        w.info.clip,
      );
      $("#trim-slot").append(trim.el);
    }
  }
  f.oninput = (e) => {
    const tab = editTabs.of(e.target);
    if (tab) tab.querySelector(".dirty").hidden = false;
  };
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
      const th = thumbs.value();
      if (th.file) {
        await invoke(
          "thumbnail_image",
          new Uint8Array(await th.file.arrayBuffer()),
          { headers: { id: w.id } },
        );
      }
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
          thumbnail: th.choice,
        },
      });
      d.close("save");
      await refresh();
    } catch (err) {
      $("#edit-error").textContent = String(err);
      $("#edit-error").hidden = false;
      showEditField(document.querySelector(editField(err)));
    }
  };
}
