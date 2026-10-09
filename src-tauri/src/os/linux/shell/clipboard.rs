pub fn clipboard_text() -> Result<String, String> {
    let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
    Ok(clipboard
        .wait_for_text()
        .map(|t| t.to_string())
        .unwrap_or_default())
}
