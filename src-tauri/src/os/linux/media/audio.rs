use super::*;

pub fn other_audio_playing() -> bool {
    false
}

pub fn loopback(
    _wanted: &std::sync::atomic::AtomicBool,
    _each: &mut dyn FnMut(&[f32]),
) -> Result<(), String> {
    Err(NOT_YET.into())
}
