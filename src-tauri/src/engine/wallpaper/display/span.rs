use super::*;

pub(in crate::engine::wallpaper) fn span_rect(rects: &[RECT]) -> RECT {
    rects
        .iter()
        .fold(rects.first().copied().unwrap_or_default(), |a, r| RECT {
            left: a.left.min(r.left),
            top: a.top.min(r.top),
            right: a.right.max(r.right),
            bottom: a.bottom.max(r.bottom),
        })
}

pub(in crate::engine::wallpaper) fn span_state(decisions: &[(State, Reason)]) -> (State, Reason) {
    decisions
        .iter()
        .find(|(st, _)| *st == State::Play)
        .or(decisions.first())
        .copied()
        .unwrap_or((State::Play, Reason::None))
}

pub(in crate::engine::wallpaper) fn spanned(core: &Core, i: usize) -> bool {
    core.settings.span && i > 0 && !os::span_per_output()
}

pub(in crate::engine::wallpaper) fn slice(rects: &[RECT], i: usize) -> Option<[i32; 4]> {
    let all = span_rect(rects);
    let r = rects.get(i)?;
    Some([
        r.left - all.left,
        r.top - all.top,
        all.right - all.left,
        all.bottom - all.top,
    ])
}
