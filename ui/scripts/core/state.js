let strings = {};
let state = null;
let selected = null;

const lang = () => state?.settings?.language || "en";
const wpTitle = (w) => shownTitle(w, lang());
const title = (id) => {
  const w = state.library.find((w) => w.id === id);
  return w ? wpTitle(w) : id;
};
const kindOf = (id) => state.library.find((w) => w.id === id)?.kind;
const selectedDisplay = () => selected;

const filters = (() => {
  try {
    return JSON.parse(localStorage.getItem("filters")) || {};
  } catch {
    return {};
  }
})();
const saveFilters = () => {
  try {
    localStorage.setItem("filters", JSON.stringify(filters));
  } catch {}
};
const categoryName = (c) => t(`category.${c}`);
