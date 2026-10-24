//! What every page shares.

pub struct State {
    pub customers: Vec<String>,
}

impl State {
    pub fn sample() -> Self {
        Self {
            customers: vec!["Ana Pérez".into(), "Bruno Díaz".into(), "Carla Soto".into()],
        }
    }
}
