pub mod session_state;

#[derive(Clone)]
pub struct AppState<Tg> {
    pub telegram: Tg,
}

impl<Tg> AppState<Tg> {
    pub fn new(telegram: Tg) -> Self {
        Self { telegram }
    }
}
