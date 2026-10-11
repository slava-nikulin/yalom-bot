use async_openai::{
    Client,
    config::OpenAIConfig,
    error::OpenAIError,
    types::responses::{CreateResponseArgs, ReasoningArgs, ReasoningEffort},
};

#[derive(thiserror::Error, Debug)]
pub enum AdapterError {
    #[error("failed to execute the request")]
    OpenAi(#[from] OpenAIError),

    #[error("empty response")]
    EmptyResponse(),
}

pub struct LlmResponse {
    pub text: String,
}

pub trait LlmAdapter {
    fn execute(
        &self,
        instructions: String,
    ) -> impl std::future::Future<Output = Result<LlmResponse, AdapterError>> + Send;
}

pub struct OpenAiAdapter {
    client: Client<OpenAIConfig>,
}

impl OpenAiAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::with_config(OpenAIConfig::new().with_api_key(api_key)),
        }
    }
}

impl LlmAdapter for OpenAiAdapter {
    async fn execute(&self, instructions: String) -> Result<LlmResponse, AdapterError> {
        let request = CreateResponseArgs::default()
            .model("gpt-6-luna")
            .reasoning(
                ReasoningArgs::default()
                    .effort(ReasoningEffort::Medium)
                    .build()?,
            )
            .instructions(instructions)
            .max_output_tokens(1024u32)
            .build()?;

        let response = self.client.responses().create(request).await?;

        Ok(LlmResponse {
            text: response
                .output_text()
                .ok_or_else(AdapterError::EmptyResponse)?,
        })
    }
}
