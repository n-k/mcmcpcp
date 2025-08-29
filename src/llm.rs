pub struct LlmClient {
    api_url: String,
    api_key: String,
}

impl LlmClient {
    pub fn new(api_url: String, api_key: String) -> Self {
        Self { api_url, api_key }
    }

    pub async fn stream(&self) {}
}
