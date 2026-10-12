use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MenuState {
    pub paused: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateMenuRequest {
    pub paused: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NudgeResponse {
    pub text: String,
}

#[derive(Deserialize)]
pub struct GenerateNudgeRequest {
    pub timezone: String,
}
