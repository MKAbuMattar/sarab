function shownTitle(w, lang) {
  return (lang && w.info.titles?.[lang]) || w.info.title || w.id;
}

function shownDescription(w, lang) {
  return (lang && w.info.descriptions?.[lang]) || w.info.description || "";
}

function filterLibrary(
  items,
  {
    type = "all",
    category = "all",
    query = "",
    sort = "name",
    label = (k) => k,
    lang = "",
  } = {},
) {
  const words = query.toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const text = (w) => {
    const c = w.info.category;
    return [
      w.info.title,
      w.info.description,
      w.id,
      c,
      c && label(c),
      ...(w.info.tags || []),
      ...Object.values(w.info.titles || {}),
      ...Object.values(w.info.descriptions || {}),
    ]
      .filter(Boolean)
      .join(" ")
      .toLocaleLowerCase();
  };
  const title = (w) => shownTitle(w, lang);
  const order =
    {
      name: (a, b) =>
        title(a).localeCompare(title(b), undefined, {
          sensitivity: "base",
          numeric: true,
        }),
      newest: (a, b) => (b.added || 0) - (a.added || 0),
      oldest: (a, b) => (a.added || 0) - (b.added || 0),
    }[sort] ?? (() => 0);
  return items
    .filter((w) => type === "all" || w.kind === type)
    .filter(
      (w) => category === "all" || (w.info.category || "none") === category,
    )
    .filter((w) => {
      const t = text(w);
      return words.every((x) => t.includes(x));
    })
    .sort(order);
}

if (typeof module !== "undefined")
  module.exports = { filterLibrary, shownTitle, shownDescription };
