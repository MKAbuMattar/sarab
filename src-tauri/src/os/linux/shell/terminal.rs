pub fn set_on_path(_add: bool) -> Result<(), String> {
    Ok(())
}

pub fn tell_terminal(msg: &str) {
    eprintln!("{msg}");
}
