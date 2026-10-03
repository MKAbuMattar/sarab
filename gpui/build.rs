fn main() {
    // Resource 1 is the icon GPUI gives every window, and Explorer shows for sarab.exe.
    embed_resource::compile("sarab.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
