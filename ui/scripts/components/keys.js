document.addEventListener(
  "keydown",
  (e) => {
    const k = e.key.toLowerCase();
    const browser =
      ["F5", "F7", "BrowserBack", "BrowserForward", "BrowserRefresh"].includes(
        e.key,
      ) ||
      (e.ctrlKey && ["r", "p", "s", "u", "f", "g", "j", "h"].includes(k)) ||
      (e.altKey && ["ArrowLeft", "ArrowRight", "Home"].includes(e.key));
    const selectAll =
      e.ctrlKey &&
      k === "a" &&
      !e.target.closest?.("input, textarea, [contenteditable]");
    if (browser || selectAll) e.preventDefault();
  },
  true,
);
