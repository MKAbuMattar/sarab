function textItems(field) {
  const editable = !field.readOnly && !field.disabled;
  const selected = field.selectionStart !== field.selectionEnd;
  const write = (text) => document.execCommand("insertText", false, text);
  return [
    {
      glyph: "",
      label: t("menu.undo"),
      keys: "Ctrl+Z",
      disabled: !editable,
      run: () => document.execCommand("undo"),
    },
    {
      glyph: "",
      label: t("menu.cut"),
      keys: "Ctrl+X",
      disabled: !editable || !selected,
      run: () => document.execCommand("cut"),
    },
    {
      glyph: "",
      label: t("menu.copy"),
      keys: "Ctrl+C",
      disabled: !selected,
      run: () => document.execCommand("copy"),
    },
    {
      glyph: "",
      label: t("menu.paste"),
      keys: "Ctrl+V",
      disabled: !editable,
      run: () =>
        invoke("clipboard_text")
          .then(write)
          .catch(() => {}),
    },
    {
      glyph: "",
      label: t("menu.selectAll"),
      keys: "Ctrl+A",
      disabled: !field.value,
      run: () => field.select(),
    },
  ];
}
