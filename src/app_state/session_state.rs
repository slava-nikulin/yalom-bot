use std::sync::Arc;

use crate::security::session::SessionTokens;

#[derive(Clone)]
pub struct MiniAppState {
    pub session_tokens: Arc<SessionTokens>,
}

impl MiniAppState {
    pub fn new(session_tokens: SessionTokens) -> Self {
        Self {
            session_tokens: Arc::new(session_tokens),
        }
    }
}
