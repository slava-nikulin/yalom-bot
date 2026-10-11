use sqlx::prelude::FromRow;

#[derive(Debug, Clone, Copy, sqlx::Type)]
pub enum NudgeDayTime {
    Any,
    Morning,
    Day,
    Evening,
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
pub enum NudgeIntent {
    Observation,
    Question,
    Invitation,
    Greeting,
}

#[derive(Debug, FromRow)]
pub struct Nudge {
    pub text: String,
    pub day_time: NudgeDayTime,
    pub kind: NudgeIntent,
}
