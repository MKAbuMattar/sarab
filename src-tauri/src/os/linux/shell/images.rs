use super::*;

pub fn shell_thumbnail(_src: &Path, _dest: &Path, _size: i32) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn shrink_png(_src: &Path, _dest: &Path, _max: u32) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn crop_png(_src: &Path, _dest: &Path, _rect: [u32; 4], _max: u32) -> Result<(), String> {
    Err(NOT_YET.into())
}
