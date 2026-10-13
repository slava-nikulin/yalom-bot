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

#[derive(Serialize)]
pub struct ErrorBody {
    pub code: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}
