use super::*;
use gtk::gio::prelude::*;

pub fn recycle(path: &Path) -> Result<(), String> {
    gtk::gio::File::for_path(path)
        .trash(None::<&gtk::gio::Cancellable>)
        .map_err(|e| format!("could not move {} to the trash: {e}", path.display()))
}
