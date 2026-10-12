use super::*;
use crate::nudge::open_ai_adapter::LlmResponse;
use std::{
    collections::VecDeque,
    mem::discriminant,
    range::Range,
    sync::{Arc, Mutex},
};

// ---------- Fakes ----------

struct FakeRng {
    rolls: VecDeque<u32>,
}

impl FakeRng {
    fn new(rolls: impl IntoIterator<Item = u32>) -> Self {
        Self {
            rolls: rolls.into_iter().collect(),
        }
    }
}

impl RandomRange for FakeRng {
    fn random_range(&mut self, _range: Range<u32>) -> u32 {
        let roll = self.rolls.pop_front().expect("unexpected RNG call");
        assert!(roll < 100);
        roll
    }
}

#[derive(Clone, Copy)]
enum StoreReply {
    Found(&'static str),
    Missing,
    Failed,
}

#[derive(Clone)]
struct FakeStore {
    reply: StoreReply,
    calls: Arc<Mutex<Vec<(NudgeIntent, NudgeDayTime)>>>,
}

impl FakeStore {
    fn new(reply: StoreReply) -> Self {
        Self {
            reply,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl NudgeStore for FakeStore {
    async fn get_random_nudge(
        &self,
        intent: NudgeIntent,
        daytime: NudgeDayTime,
    ) -> Result<Option<String>, sqlx::Error> {
        self.calls.lock().unwrap().push((intent, daytime));

        match self.reply {
            StoreReply::Found(text) => Ok(Some(text.to_owned())),
            StoreReply::Missing => Ok(None),
            StoreReply::Failed => Err(sqlx::Error::PoolClosed),
        }
    }
}

#[derive(Clone, Copy)]
enum LlmReply {
    Text(&'static str),
    Failed,
}

#[derive(Clone)]
struct FakeLlm {
    reply: LlmReply,
    requests: Arc<Mutex<Vec<LlmRequest>>>,
}

impl FakeLlm {
    fn new(reply: LlmReply) -> Self {
        Self {
            reply,
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl LlmAdapter for FakeLlm {
    async fn execute(&self, request: LlmRequest) -> Result<LlmResponse, AdapterError> {
        self.requests.lock().unwrap().push(request);

        match self.reply {
            LlmReply::Text(text) => Ok(LlmResponse {
                text: text.to_owned(),
            }),
            LlmReply::Failed => Err(AdapterError::NotCompletedResponse),
        }
    }
}

// ---------- Helpers ----------

fn test_service(
    store_reply: StoreReply,
    llm_reply: LlmReply,
) -> (NudgeService<FakeLlm, FakeStore>, FakeStore, FakeLlm) {
    let store = FakeStore::new(store_reply);
    let llm = FakeLlm::new(llm_reply);

    let service = NudgeService::new(llm.clone(), store.clone());

    (service, store, llm)
}

fn now(utc_hour: u32) -> DateTime<Tz> {
    DateTime::parse_from_rfc3339(&format!("2026-10-12T{utc_hour:02}:00:00Z"))
        .unwrap()
        .with_timezone(&chrono_tz::Asia::Almaty)
}

fn assert_store_called(store: &FakeStore, intent: NudgeIntent, daytime: NudgeDayTime) {
    let calls = store.calls.lock().unwrap();

    assert_eq!(calls.len(), 1);
    assert_eq!(discriminant(&calls[0].0), discriminant(&intent));
    assert_eq!(discriminant(&calls[0].1), discriminant(&daytime));
}

// ---------- Tests ----------

#[tokio::test]
async fn curated_returns_text_without_calling_llm() {
    // Asia/Almaty = UTC+5.
    // Проверяем Morning, Day и Evening.
    for (utc_hour, daytime) in [
        (3, NudgeDayTime::Morning),
        (7, NudgeDayTime::Day),
        (13, NudgeDayTime::Evening),
    ] {
        let (service, store, llm) =
            test_service(StoreReply::Found("Curated text"), LlmReply::Failed);

        // time_focus = true
        // intent = Observation
        // curated = true
        let rng = FakeRng::new([49, 0, 19]);

        let result = service.generate(rng, now(utc_hour)).await.unwrap();

        assert_eq!(result.text, "Curated text");
        assert!(matches!(result.source, NudgeSource::Curated));
        assert!(matches!(result.intent, NudgeIntent::Observation));
        assert!(result.time_focus);

        assert_store_called(&store, NudgeIntent::Observation, daytime);
        assert_eq!(llm.requests.lock().unwrap().len(), 0);
    }
}

#[tokio::test]
async fn curated_without_time_focus_uses_any() {
    let (service, store, llm) =
        test_service(StoreReply::Found("General question"), LlmReply::Failed);

    // time_focus = false
    // intent = Question
    // curated = true
    let rng = FakeRng::new([50, 60, 0]);

    let result = service.generate(rng, now(3)).await.unwrap();

    assert_eq!(result.text, "General question");
    assert!(matches!(result.source, NudgeSource::Curated));
    assert!(!result.time_focus);

    assert_store_called(&store, NudgeIntent::Question, NudgeDayTime::Any);

    assert_eq!(llm.requests.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn llm_generation_skips_store() {
    let (service, store, llm) = test_service(StoreReply::Failed, LlmReply::Text("Hello!"));

    // time_focus = false
    // intent = Greeting
    // curated = false
    let rng = FakeRng::new([50, 90, 20]);

    let result = service.generate(rng, now(3)).await.unwrap();

    assert_eq!(result.text, "Hello!");
    assert!(matches!(result.source, NudgeSource::OpenAi));
    assert!(matches!(result.intent, NudgeIntent::Greeting));
    assert!(!result.time_focus);

    assert_eq!(store.calls.lock().unwrap().len(), 0);

    let requests = llm.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);

    let request = &requests[0];

    assert_eq!(
        request.instructions,
        [
            IDENTITY_PROMPT,
            EXAMPLES_PROMPT,
            MESSAGE_PROMPT,
            GREETING_INTENT_PROMPT,
        ]
        .join("\n")
    );

    assert!(
        request
            .context
            .contains("Current date and time: 2026-10-12T08:00:00+05:00")
    );
    assert!(request.context.contains("Timezone: Asia/Almaty"));
    assert!(!request.context.contains(TIME_FOCUS_PROMPT));
}

#[tokio::test]
async fn curated_miss_falls_back_to_llm() {
    let (service, store, llm) =
        test_service(StoreReply::Missing, LlmReply::Text("Generated invitation"));

    // time_focus = true
    // intent = Invitation
    // curated = true
    let rng = FakeRng::new([0, 30, 0]);

    let result = service.generate(rng, now(3)).await.unwrap();

    assert_eq!(result.text, "Generated invitation");
    assert!(matches!(result.source, NudgeSource::OpenAi));
    assert!(matches!(result.intent, NudgeIntent::Invitation));
    assert!(result.time_focus);

    assert_store_called(&store, NudgeIntent::Invitation, NudgeDayTime::Morning);

    let requests = llm.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);

    assert!(requests[0].instructions.ends_with(INVITATION_INTENT_PROMPT));
    assert_eq!(requests[0].context.matches(TIME_FOCUS_PROMPT).count(), 1);
}

#[tokio::test]
async fn store_error_is_propagated() {
    let (service, store, llm) = test_service(
        StoreReply::Failed,
        LlmReply::Text("Should not be generated"),
    );

    // time_focus = true
    // intent = Question
    // curated = true
    let rng = FakeRng::new([0, 60, 0]);

    let result = service.generate(rng, now(3)).await;

    assert!(matches!(
        result,
        Err(NudgeServiceError::StoreError(sqlx::Error::PoolClosed))
    ));

    assert_store_called(&store, NudgeIntent::Question, NudgeDayTime::Morning);

    // Ошибка БД не маскируется fallback на LLM.
    assert_eq!(llm.requests.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn llm_error_is_propagated() {
    let (service, store, llm) =
        test_service(StoreReply::Found("Should not be used"), LlmReply::Failed);

    let rng = FakeRng::new([50, 0, 20]);

    let result = service.generate(rng, now(3)).await;

    assert!(matches!(
        result,
        Err(NudgeServiceError::OpenAiAdapterError(
            AdapterError::NotCompletedResponse
        ))
    ));

    assert_eq!(store.calls.lock().unwrap().len(), 0);
    assert_eq!(llm.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn inappropriate_time_rejects_generation() {
    for utc_hour in [0, 20] {
        // В Алматы это 05:00 и 01:00 соответственно.
        let (service, store, llm) =
            test_service(StoreReply::Found("Curated"), LlmReply::Text("Generated"));

        // RNG не должен вызываться вообще.
        let rng = FakeRng::new([]);

        let result = service.generate(rng, now(utc_hour)).await;

        assert!(matches!(result, Err(NudgeServiceError::InappropriateTime)));

        assert_eq!(store.calls.lock().unwrap().len(), 0);
        assert_eq!(llm.requests.lock().unwrap().len(), 0);
    }
}

#[tokio::test]
async fn intent_probability_boundaries() {
    for (roll, expected_intent, expected_prompt) in [
        (0, NudgeIntent::Observation, OBSERVATION_INTENT_PROMPT),
        (29, NudgeIntent::Observation, OBSERVATION_INTENT_PROMPT),
        (30, NudgeIntent::Invitation, INVITATION_INTENT_PROMPT),
        (59, NudgeIntent::Invitation, INVITATION_INTENT_PROMPT),
        (60, NudgeIntent::Question, QUESTION_INTENT_PROMPT),
        (89, NudgeIntent::Question, QUESTION_INTENT_PROMPT),
        (90, NudgeIntent::Greeting, GREETING_INTENT_PROMPT),
        (99, NudgeIntent::Greeting, GREETING_INTENT_PROMPT),
    ] {
        let (service, store, llm) = test_service(StoreReply::Failed, LlmReply::Text("Generated"));

        // time_focus = false, curated = false
        let rng = FakeRng::new([50, roll, 20]);

        let result = service.generate(rng, now(3)).await.unwrap();

        assert_eq!(
            discriminant(&result.intent),
            discriminant(&expected_intent),
            "unexpected intent for roll {roll}"
        );

        assert_eq!(store.calls.lock().unwrap().len(), 0);

        let requests = llm.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].instructions.ends_with(expected_prompt));
    }
}

#[test]
fn daytime_boundaries() {
    for hour in [0, 5] {
        assert!(get_daytime(hour).is_none());
    }

    for hour in [6, 11] {
        assert!(matches!(get_daytime(hour), Some(NudgeDayTime::Morning)));
    }

    for hour in [12, 17] {
        assert!(matches!(get_daytime(hour), Some(NudgeDayTime::Day)));
    }

    for hour in [18, 23] {
        assert!(matches!(get_daytime(hour), Some(NudgeDayTime::Evening)));
    }
}
