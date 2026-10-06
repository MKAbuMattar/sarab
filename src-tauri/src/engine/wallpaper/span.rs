//! One wallpaper stretched across every display.

use super::*;

/// The rectangle that holds every display, in screen coordinates.
pub(super) fn span_rect(rects: &[RECT]) -> RECT {
    rects
        .iter()
        .fold(rects.first().copied().unwrap_or_default(), |a, r| RECT {
            left: a.left.min(r.left),
            top: a.top.min(r.top),
            right: a.right.max(r.right),
            bottom: a.bottom.max(r.bottom),
        })
}

/// A spanned wallpaper plays while any display can play it, and otherwise rests for the
/// reason the first display gives.
pub(super) fn span_state(decisions: &[(State, Reason)]) -> (State, Reason) {
    decisions
        .iter()
        .find(|(st, _)| *st == State::Play)
        .or(decisions.first())
        .copied()
        .unwrap_or((State::Play, Reason::None))
}

/// Is display `i` drawn by the spanning window on display 0 rather than its own?
pub(super) fn spanned(core: &Core, i: usize) -> bool {
    core.settings.span && i > 0
}
