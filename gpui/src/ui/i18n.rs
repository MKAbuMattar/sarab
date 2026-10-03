//! UI strings, from the same JSON files the web window used.

use std::collections::HashMap;

pub struct Strings(HashMap<String, String>);

impl Strings {
    pub fn load(lang: &str) -> Strings {
        let file = match lang {
            "ar" => include_str!("../../assets/i18n/ar.json"),
            _ => include_str!("../../assets/i18n/en.json"),
        };
        Strings(serde_json::from_str(file).unwrap_or_default())
    }

    /// The string for `key`, or the key itself so a missing string is visible, not blank.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.0.get(key).map_or(key, String::as_str)
    }

    /// `{name}` placeholders filled from `vars`.
    pub fn fmt(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let mut s = self.get(key).to_string();
        for (k, v) in vars {
            s = s.replace(&format!("{{{k}}}"), v);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_have_the_same_keys() {
        let (en, ar) = (Strings::load("en"), Strings::load("ar"));
        let mut a: Vec<_> = en.0.keys().collect();
        let mut b: Vec<_> = ar.0.keys().collect();
        a.sort();
        b.sort();
        assert_eq!(a, b);
        assert_eq!(
            en.fmt("about.version", &[("v", "1.2.3")]).contains("1.2.3"),
            true
        );
    }
}
