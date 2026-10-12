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
