const $ = (s) => document.querySelector(s);

function el(tag, attrs = {}, ...children) {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k.startsWith("on")) n.addEventListener(k.slice(2), v);
    else if (v !== undefined) n.setAttribute(k, v);
  }
  n.append(...children);
  return n;
}
const icon = (glyph) =>
  el("i", { class: "icon", "aria-hidden": "true" }, glyph);
const btn = (cls, glyph, label, onclick) =>
  el(
    "button",
    { class: cls, type: "button", onclick },
    icon(glyph),
    el("span", {}, label),
  );

const iconBtn = (glyph, label, onclick, cls = "") =>
  el(
    "button",
    {
      class: `subtle icon-only ${cls}`,
      type: "button",
      "aria-label": label,
      title: label,
      onclick,
    },
    icon(glyph),
  );
