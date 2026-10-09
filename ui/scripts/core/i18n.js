const t = (key, vars = {}) =>
  (strings[key] ?? key).replace(/\{(\w+)\}/g, (_, k) => vars[k] ?? "");

async function loadLanguage(lang) {
  strings = await (await fetch(`i18n/${lang}.json`)).json();
  document.documentElement.lang = lang;
  document.documentElement.dir = lang === "ar" ? "rtl" : "ltr";
  document.querySelectorAll("[data-t]").forEach((el) => {
    el.textContent = t(el.dataset.t);
  });
  document.querySelectorAll("[data-tp]").forEach((el) => {
    el.placeholder = t(el.dataset.tp);
  });
  document.querySelectorAll("[data-tl]").forEach((el) => {
    el.setAttribute("aria-label", t(el.dataset.tl));
  });
  document.querySelectorAll("select").forEach((s) => s._combo?.sync());
}
