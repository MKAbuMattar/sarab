const THUMB_MAX = 10 * 1024 * 1024;

function thumbChoice(w) {
  const input = el("input", {
    type: "file",
    accept: "image/png,image/jpeg,image/webp",
    class: "picker",
    tabindex: "-1",
    "aria-hidden": "true",
  });
  const pic = (url, glyph) =>
    url ? el("img", { src: url, alt: "" }) : icon(glyph);
  const autoPic = el(
    "div",
    { class: "pic" },
    pic(w.auto_thumb_url, ICON[w.kind] ?? ""),
  );
  const minePic = el("div", { class: "pic" }, pic(w.custom_thumb_url, ""));
  const rAuto = el("input", { type: "radio", name: "thumb", value: "auto" });
  const rMine = el("input", { type: "radio", name: "thumb", value: "image" });
  const card = (r, p, label) =>
    el(
      "label",
      { class: "thumb-card" },
      p,
      el("span", {}, r, el("span", {}, label)),
    );
  const choose = el(
    "button",
    { type: "button", class: "choose" },
    icon(""),
    el("span", {}, t("thumb.choose")),
  );
  const said = el("p", { class: "caption said", "aria-live": "polite" });
  const root = el(
    "div",
    { class: "thumbs" },
    el(
      "div",
      { class: "cards", role: "radiogroup", "aria-label": t("edit.thumbnail") },
      card(rAuto, autoPic, t("thumb.auto")),
      card(rMine, minePic, t("thumb.image")),
    ),
    el(
      "div",
      { class: "row" },
      choose,
      el("span", { class: "caption" }, t("thumb.hint")),
    ),
    said,
    input,
  );

  let file = null;
  rMine.disabled = !w.custom_thumb_url;
  (w.info.thumbnail_choice === "image" && w.custom_thumb_url
    ? rMine
    : rAuto
  ).checked = true;
  const tell = () =>
    (said.textContent = t("thumb.shows", {
      s: rMine.checked ? t("thumb.yours") : t("thumb.automatic"),
    }));
  tell();
  rAuto.addEventListener("change", tell);
  rMine.addEventListener("change", tell);

  choose.onclick = () => input.click();
  input.addEventListener("change", () => {
    const f = input.files[0];
    input.value = "";
    if (!f) return;
    if (f.size > THUMB_MAX) {
      said.textContent = t("thumb.tooBig");
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      file = f;
      minePic.replaceChildren(el("img", { src: reader.result, alt: "" }));
      rMine.disabled = false;
      rMine.checked = true;
      tell();
      root.dispatchEvent(new Event("input", { bubbles: true }));
    };
    reader.readAsDataURL(f);
  });

  return {
    el: root,
    value: () => ({
      choice: rMine.checked ? "image" : "auto",
      file: rMine.checked ? file : null,
    }),
  };
}
