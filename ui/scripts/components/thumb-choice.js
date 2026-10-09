const THUMB_MAX = 10 * 1024 * 1024;

function thumbChoice(w, opts = {}) {
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
  const framePic = el("div", { class: "pic" }, pic(w.frame_thumb_url, ""));
  const radio = (value) => el("input", { type: "radio", name: "thumb", value });
  const rAuto = radio("auto"),
    rMine = radio("image"),
    rFrame = radio("frame");
  const frameLabel = el("span", {});
  const card = (r, p, label) =>
    el(
      "label",
      { class: "thumb-card" },
      p,
      el(
        "span",
        {},
        r,
        typeof label === "string" ? el("span", {}, label) : label,
      ),
    );
  const choose = el(
    "button",
    { type: "button", class: "choose" },
    icon(""),
    el("span", {}, t("thumb.choose")),
  );
  const said = el("p", { class: "caption said", "aria-live": "polite" });
  const withFrame = opts.capture && w.info.type === "video";
  const root = el(
    "div",
    { class: "thumbs" },
    el(
      "div",
      {
        class: `cards${withFrame ? " three" : ""}`,
        role: "radiogroup",
        "aria-label": t("edit.thumbnail"),
      },
      card(rAuto, autoPic, t("thumb.auto")),
      ...(withFrame ? [card(rFrame, framePic, frameLabel)] : []),
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

  let file = null,
    time = w.info.thumbnail_time ?? null,
    staged = false;
  const labelFrame = () =>
    (frameLabel.textContent =
      time === null
        ? t("thumb.frame")
        : t("thumb.frameAt", { t: fmtTime(time) }));
  labelFrame();
  rMine.disabled = !w.custom_thumb_url;
  rFrame.disabled = !w.frame_thumb_url;
  const saved = w.info.thumbnail_choice;
  (saved === "image" && w.custom_thumb_url
    ? rMine
    : saved === "frame" && w.frame_thumb_url && withFrame
      ? rFrame
      : rAuto
  ).checked = true;
  const tell = () =>
    (said.textContent = t("thumb.shows", {
      s: rMine.checked
        ? t("thumb.yours")
        : rFrame.checked
          ? t("thumb.frameAt", { t: fmtTime(time ?? 0) })
          : t("thumb.automatic"),
    }));
  tell();
  for (const r of [rAuto, rMine, rFrame]) r.addEventListener("change", tell);
  const changed = () =>
    root.dispatchEvent(new Event("input", { bubbles: true }));

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
      changed();
    };
    reader.readAsDataURL(f);
  });

  return {
    el: root,
    frameBusy(at) {
      time = at;
      staged = false;
      labelFrame();
      framePic.replaceChildren(
        el("span", { class: "caption" }, t("thumb.capturing")),
      );
      said.textContent = t("thumb.capturing");
    },
    frameReady(url) {
      staged = true;
      framePic.replaceChildren(el("img", { src: url, alt: "" }));
      rFrame.disabled = false;
      rFrame.checked = true;
      tell();
      changed();
    },
    frameFailed(reason) {
      framePic.replaceChildren(icon(""));
      said.textContent = `${t("thumb.frameFailed")} ${reason ?? ""}`.trim();
    },
    frameClear() {
      time = null;
      staged = false;
      labelFrame();
      framePic.replaceChildren(icon(""));
      if (rFrame.checked) rAuto.checked = true;
      rFrame.disabled = true;
      tell();
      changed();
    },
    value: () => ({
      choice: rMine.checked ? "image" : rFrame.checked ? "frame" : "auto",
      file: rMine.checked ? file : null,
      time: rFrame.checked ? time : null,
      staged: rFrame.checked && staged,
    }),
  };
}
