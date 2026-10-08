#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub key: String,
    pub rect: super::RECT,
    pub work: super::RECT,
}

pub fn monitors() -> Vec<Monitor> {
    Vec::new()
}
