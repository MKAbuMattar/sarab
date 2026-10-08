use super::RECT;

#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub key: String,
    pub rect: RECT,
    pub work: RECT,
}

fn rect(r: gdk::Rectangle) -> RECT {
    RECT {
        left: r.x(),
        top: r.y(),
        right: r.x() + r.width(),
        bottom: r.y() + r.height(),
    }
}

pub fn monitors() -> Vec<Monitor> {
    let Some(display) = gdk::Display::default() else {
        return Vec::new();
    };
    (0..display.n_monitors())
        .filter_map(|i| display.monitor(i))
        .map(|m| {
            let r = rect(m.geometry());
            let model = m.model().map(|s| s.to_string()).unwrap_or_default();
            Monitor {
                key: format!("{model}@{},{}", r.left, r.top),
                rect: r,
                work: rect(m.workarea()),
            }
        })
        .collect()
}
