const lessMotion = matchMedia("(prefers-reduced-motion: reduce)");
function thumbFor(w) {
  const still = () =>
    w.thumb_url
      ? el("img", { src: w.thumb_url, alt: "", loading: "lazy" })
      : icon(ICON[w.kind] ?? "");
  const box = el("div", { class: "thumb" }, still());
  if (!w.preview_url) return box;
  box.addEventListener("pointerenter", () => {
    if (lessMotion.matches || box.querySelector(".preview")) return;
    const gif = w.kind === "gif";
    const media = gif
      ? el("img", { class: "preview", src: w.preview_url, alt: "" })
      : Object.assign(el("video", { class: "preview", src: w.preview_url }), {
          muted: true,
          autoplay: true,
          loop: true,
          playsInline: true,
        });
    media.addEventListener(
      gif ? "load" : "playing",
      () => media.classList.add("on"),
      { once: true },
    );
    box.append(media);
  });
  box.addEventListener("pointerleave", () =>
    box.querySelector(".preview")?.remove(),
  );
  return box;
}
