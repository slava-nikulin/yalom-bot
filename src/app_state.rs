use std::sync::Arc;

use crate::security::SessionTokens;

#[derive(Clone)]
pub struct AppState<Tg> {
    pub telegram: Tg,
    pub session_tokens: Arc<SessionTokens>,
}

impl<Tg> AppState<Tg> {
    pub fn new(telegram: Tg, session_tokens: SessionTokens) -> Self {
        Self {
            telegram,
            session_tokens: Arc::new(session_tokens),
        }
    }
}
