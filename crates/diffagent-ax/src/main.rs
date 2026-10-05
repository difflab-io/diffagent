use diffagent_core::{Backend, Result};
use serde_json::json;

struct Ax;

impl Backend for Ax {
    fn name(&self) -> &'static str {
        "ax"
    }

    async fn generate(&self, model: &str, prompt: &str) -> Result<String> {
        let model = model.to_owned();
        let prompt = prompt.to_owned();
        // Ax's HTTP client blocks; keep it off Tokio's async executor threads.
        tokio::task::spawn_blocking(move || -> std::result::Result<String, String> {
            use axllm::{AxAIClient, ai};
            let key = std::env::var("DEEPSEEK_API_KEY").map_err(|e| e.to_string())?;
            let mut client = ai(
                "deepseek",
                json!({
                    "api_key": key,
                    "base_url": "https://api.deepseek.com/v1",
                    "model": model,
                }),
            )
            .map_err(|e| e.to_string())?;
            let response = client
                .chat(json!({
                    "chat_prompt": [{"role": "user", "content": prompt}]
                }))
                .map_err(|e| e.to_string())?;
            response["results"][0]["content"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "Ax returned no text".to_owned())
        })
        .await?
        .map_err(Into::into)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let input = diffagent_core::input_from_args()?;
    let (dir, passed) = diffagent_core::run(&Ax, &input).await?;
    println!(
        "Ax: {} | artifacts: {}",
        if passed { "PASS" } else { "FAIL" },
        dir.display()
    );
    if !passed {
        return Err("evaluation failed; inspect artifacts".into());
    }
    Ok(())
}
