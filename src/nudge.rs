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
        now: DateTime<Utc>,
        tz: Tz,
    ) -> Result<GeneratedNudge, NudgeServiceError> {
        let now_tz = now.with_timezone(&tz);

        let intent = match rng.random_range((0..100).into()) {
            0..30 => NudgeIntent::Observation,
            30..60 => NudgeIntent::Invitation,
            60..90 => NudgeIntent::Question,
            _ => NudgeIntent::Greeting,
        };

        let time_focus = matches!(rng.random_range((0..100).into()), 0..50);

        let mut source = NudgeSource::OpenAi;
        let is_curated_source = rng.random_range((0..100).into()) < 20;

        let curated_text = if is_curated_source {
            source = NudgeSource::Curated;
            let daytime = if time_focus {
                get_daytime(now_tz.hour())
            } else {
                None
            };
            self.store
                .get_random_nudge(intent, daytime)
                .await
                .map_err(NudgeServiceError::StoreError)?
        } else {
            None
        };

        let text = if let Some(curated_text_val) = curated_text {
            curated_text_val
        } else {
            self.open_ai_generate(intent, now_tz, time_focus).await?
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
        intent: NudgeIntent,
        now_tz: DateTime<Tz>,
        time_focus: bool,
    ) -> Result<String, NudgeServiceError> {
        let llm_request = build_llm_request(intent, now_tz, time_focus);

        let llm_resp = self
            .open_ai_adapter
            .execute(llm_request)
            .await
            .map_err(OpenAiAdapterError)?;

        Ok(llm_resp.text)
    }
}

fn build_llm_request(intent: NudgeIntent, now_tz: DateTime<Tz>, time_focus: bool) -> LlmRequest {
    let instructions = [
        IDENTITY_PROMPT,
        EXAMPLES_PROMPT,
        ON_DEMAND_MODE_PROMPT,
        get_intent_prompt(intent),
    ]
    .join("\n");

    let mut context = format!(
        "Current date and time: {}\nTimezone: {}",
        now_tz.to_rfc3339_opts(SecondsFormat::Secs, true),
        now_tz.timezone(),
    );
    if time_focus {
        context.push('\n');
        context.push_str(TIME_FOCUS_PROMPT);
    }

    LlmRequest {
        instructions,
        context,
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

fn get_intent_prompt(intent: NudgeIntent) -> &'static str {
    match intent {
        NudgeIntent::Observation => OBSERVATION_INTENT_PROMPT,
        NudgeIntent::Question => QUESTION_INTENT_PROMPT,
        NudgeIntent::Invitation => INVITATION_INTENT_PROMPT,
        NudgeIntent::Greeting => GREETING_INTENT_PROMPT,
    }
}
