//! Arabic for GPUI. GPUI lays every run out left to right (DirectWrite with bidi level 0), so
//! Arabic arrives joined but backwards. Here each line is shaped into presentation forms, which
//! need no further joining, then reversed into visual order with the Unicode bidi algorithm.

use unicode_bidi::{BidiInfo, Level};

/// About this many characters fit a setting description; longer text is broken here, before
/// reordering, so wrapped lines still read top to bottom.
// ponytail: fixed character budget, measure with the text system if narrow windows wrap again.
const LINE: usize = 56;

fn is_rtl(c: char) -> bool {
    matches!(c, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
}

fn lines(text: &str) -> Vec<String> {
    let mut out = vec![];
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > LINE {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        out.push(line);
    }
    out
}

/// Text as GPUI must draw it. Unchanged when it has no right-to-left letters.
pub fn visual(text: &str) -> String {
    if !text.chars().any(is_rtl) {
        return text.to_string();
    }
    lines(text)
        .iter()
        .map(|l| {
            let shaped = ar_reshaper::reshape_line(l);
            let info = BidiInfo::new(&shaped, Some(Level::rtl()));
            match info.paragraphs.first() {
                Some(p) => info.reorder_line(p, p.range.clone()).into_owned(),
                None => shaped,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_is_shaped_and_reordered() {
        // سراب: seen left to right, ب (isolated after alef) comes first, ر joins س as its final form, س (initial) is last.
        let v: Vec<char> = visual("سراب").chars().collect();
        assert_eq!(v, ['\u{FE8F}', '\u{FE8D}', '\u{FEAE}', '\u{FEB3}']);
        // Latin and numbers inside Arabic keep their own order.
        assert!(visual("الإصدار 0.0.4").contains("0.0.4"));
        assert_eq!(visual("Sarab"), "Sarab");
        // Long text breaks into lines before reordering, so each line is reversed on its own.
        let long = "كلمة ".repeat(30);
        assert!(visual(long.trim()).lines().count() > 1);
        assert!(
            visual(long.trim())
                .lines()
                .all(|l| l.chars().count() <= LINE)
        );
    }
}
