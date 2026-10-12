use std::range::Range;

use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use chrono_tz::Tz;

use crate::nudge::{
    NudgeServiceError::OpenAiAdapterError,
    model::{NudgeDayTime, NudgeIntent},
    open_ai_adapter::{AdapterError, LlmAdapter, LlmRequest},
    store::NudgeStore,
};

pub mod model;
pub mod open_ai_adapter;
pub mod store;

const IDENTITY_PROMPT: &str = include_str!("../agent/prompts/identity.md");
const EXAMPLES_PROMPT: &str = include_str!("../agent/prompts/examples.md");
const ON_DEMAND_MODE_PROMPT: &str = include_str!("../agent/prompts/on_demand.md");

const GREETING_INTENT_PROMPT: &str = include_str!("../agent/prompts/intents/greeting.md");
const INVITATION_INTENT_PROMPT: &str = include_str!("../agent/prompts/intents/invitation.md");
const OBSERVATION_INTENT_PROMPT: &str = include_str!("../agent/prompts/intents/observation.md");
const QUESTION_INTENT_PROMPT: &str = include_str!("../agent/prompts/intents/question.md");

// const NO_TIME_PROMPT: &str =
// "Время суток неизвестно. Не привязывай сообщение к определённому времени суток.";
const TIME_FOCUS_PROMPT: &str = "Тематический фокус: сообщение должно быть непосредственно связано с текущим временем суток. Свободно выбери конкретную мысль, сохраняя заданное намерение и характер Yalom.";

#[derive(Debug, thiserror::Error)]
pub enum NudgeServiceError {
    #[error("openAI adapter error")]
    OpenAiAdapterError(#[source] AdapterError),

    #[error("nudge store error")]
    StoreError(#[source] sqlx::Error),
}

pub enum NudgeSource {
    OpenAi,
    Curated,
}

pub enum NudgeMode {
    OnDemand,
    Schedule,
}

pub trait RandomRange {
    fn random_range(&mut self, range: Range<u32>) -> u32;
}

pub struct GeneratedNudge {
    pub text: String,
    pub intent: NudgeIntent,
    pub source: NudgeSource,
    pub time_focus: bool,
}
pub struct NudgeService<A, S>
where
    A: LlmAdapter,
    S: NudgeStore,
{
    open_ai_adapter: A,
    store: S,
}

impl<A, S> NudgeService<A, S>
where
    A: LlmAdapter,
    S: NudgeStore,
{
    pub fn new(open_ai_adapter: A, store: S) -> Self {
        Self {
            open_ai_adapter,
            store,
        }
    }

    pub async fn generate<R: RandomRange>(
        &self,
        mut rng: R,
        mode: NudgeMode,
        now: DateTime<Utc>,
        tz: Tz,
    ) -> Result<GeneratedNudge, NudgeServiceError> {
        assert!(matches!(mode, NudgeMode::OnDemand));

        let now_tz = now.with_timezone(&tz);

        let mode_prompt = match mode {
            NudgeMode::OnDemand => ON_DEMAND_MODE_PROMPT,
            NudgeMode::Schedule => unreachable!("will be implemented later"),
        };

        let intent = match rng.random_range((0..100).into()) {
            0..30 => NudgeIntent::Observation,
            30..60 => NudgeIntent::Invitation,
            60..90 => NudgeIntent::Question,
            _ => NudgeIntent::Greeting,
        };

        let time_focus = matches!(rng.random_range((0..100).into()), 0..50);

        let mut source = NudgeSource::OpenAi;
        let text = match rng.random_range((0..100).into()) {
            0..80 => {
                self.open_ai_generate(tz, now_tz, mode_prompt, intent, time_focus)
                    .await?
            }
            _ => {
                let daytime = if time_focus {
                    get_daytime(now_tz.hour())
                } else {
                    None
                };
                let curated_nudge = self
                    .store
                    .get_random_nudge(intent, daytime)
                    .await
                    .map_err(NudgeServiceError::StoreError)?;

                if let Some(nudge) = curated_nudge {
                    source = NudgeSource::Curated;
                    nudge
                } else {
                    self.open_ai_generate(tz, now_tz, mode_prompt, intent, time_focus)
                        .await?
                }
            }
        };

        Ok(GeneratedNudge {
            text,
            intent,
            source,
            time_focus,
        })
    }

    async fn open_ai_generate(
        &self,
        tz: Tz,
        now_tz: DateTime<Tz>,
        mode_prompt: &str,
        intent: NudgeIntent,
        time_focus: bool,
    ) -> Result<String, NudgeServiceError> {
        let mut instructions = String::new();
        let mut context = String::new();
        instructions.push_str(IDENTITY_PROMPT);
        instructions.push('\n');
        instructions.push_str(EXAMPLES_PROMPT);
        instructions.push('\n');
        instructions.push_str(mode_prompt);
        instructions.push('\n');
        instructions.push_str(match intent {
            NudgeIntent::Observation => OBSERVATION_INTENT_PROMPT,
            NudgeIntent::Question => QUESTION_INTENT_PROMPT,
            NudgeIntent::Invitation => INVITATION_INTENT_PROMPT,
            NudgeIntent::Greeting => GREETING_INTENT_PROMPT,
        });
        instructions.push('\n');

        context.push_str(TIME_FOCUS_PROMPT);
        context.push('\n');
        context.push_str("Current date and time:");
        context.push_str(&now_tz.to_rfc3339_opts(SecondsFormat::Secs, true));
        context.push('\n');
        context.push_str("Timezone: ");
        context.push_str(&tz.to_string());
        if time_focus {
            context.push_str(TIME_FOCUS_PROMPT);
        };

        let llm_resp = self
            .open_ai_adapter
            .execute(LlmRequest {
                instructions,
                context,
            })
            .await
            .map_err(OpenAiAdapterError)?;

        Ok(llm_resp.text)
    }
}

pub fn get_daytime(hour: u32) -> Option<NudgeDayTime> {
    match hour {
        6..=11 => Some(NudgeDayTime::Morning),
        12..=17 => Some(NudgeDayTime::Day),
        18..=23 => Some(NudgeDayTime::Evening),
        _ => None,
    }
}
