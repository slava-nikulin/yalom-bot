use chrono::{DateTime, Timelike};
use chrono_tz::Tz;

#[derive(Debug, Clone, Copy, sqlx::Type)]
pub enum NudgeDayTime {
    Any,
    Morning,
    Day,
    Evening,
}

#[derive(Debug, thiserror::Error)]
pub enum NudgeTimeError {
    #[error("Inappropriate time")]
    InappropriateTime,
}

pub struct NudgeTime {
    pub now: DateTime<Tz>,
    pub daytime: NudgeDayTime,
}

impl TryFrom<DateTime<Tz>> for NudgeTime {
    type Error = NudgeTimeError;

    fn try_from(value: DateTime<Tz>) -> Result<Self, Self::Error> {
        let daytime = match value.hour() {
            6..=11 => Some(NudgeDayTime::Morning),
            12..=17 => Some(NudgeDayTime::Day),
            18..=23 => Some(NudgeDayTime::Evening),
            _ => None,
        };

        Ok(Self {
            now: value,
            daytime: daytime.ok_or(NudgeTimeError::InappropriateTime)?,
        })
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
pub enum NudgeIntent {
    Observation,
    Question,
    Invitation,
    Greeting,
}
