use super::*;

pub fn update_toast_xml(_title: &str, _body: &str, _install: &str, _later: &str) -> String {
    String::new()
}

pub fn show_toast(
    _app_id: &str,
    _xml: &str,
    _on_answer: impl Fn(String) + Send + Sync + 'static,
) -> Result<(), String> {
    Err(NOT_YET.into())
}
