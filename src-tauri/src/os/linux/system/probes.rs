use super::*;

pub fn signals(mons: &[Monitor]) -> Signals {
    Signals {
        covered: vec![false; mons.len()],
        desktop_focused: true,
        ..Default::default()
    }
}

pub fn last_input() -> (u32, u32) {
    (0, 0)
}
