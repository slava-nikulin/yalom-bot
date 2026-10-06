use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MenuState {
    pub paused: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateMenuRequest {
    pub paused: bool,
}
