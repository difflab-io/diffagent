#![allow(clippy::result_large_err)] // Ax requires AxResult<Value> from tool handlers.

use diffagent_core::{Backend, Generation, Result, Usage, tools::ToolHost};
use serde_json::{Value, json};

fn usage_from_records(records: &[&Value]) -> Usage {
    let mut input = Some(0u64);
    let mut output = Some(0u64);
    let mut cached = Some(0u64);
    for record in records {
        let tokens = record.get("tokens").unwrap_or(record);
        let prompt = tokens.get("prompt_tokens").and_then(Value::as_u64);
        let completion = tokens.get("completion_tokens").and_then(Value::as_u64);
        // Ax omits cache_read_tokens on zero-hit requests. Only infer zero if
        // total_tokens equals prompt + completion; otherwise mark it unknown.
        let hit = tokens
            .get("cache_read_tokens")
            .and_then(Value::as_u64)
            .or_else(|| {
                tokens
                    .get("total_tokens")?
                    .as_u64()?
                    .checked_sub(prompt?.checked_add(completion?)?)
            });
        input = input.zip(prompt).zip(hit).map(|((a, p), h)| a + p + h);
        output = output.zip(completion).map(|(a, c)| a + c);
        cached = cached.zip(hit).map(|(a, h)| a + h);
    }
    Usage {
        input_tokens: input,
        output_tokens: output,
        cached_input_tokens: cached,
        model_calls: records.len(),
    }
}

struct Ax;
impl Backend for Ax {
    fn name(&self) -> &'static str {
        "ax"
    }

    async fn generate(
        &self,
        model: &str,
        prompt: &str,
        host: ToolHost,
        tools_enabled: bool,
    ) -> Result<Generation> {
        let model = model.to_owned();
        let prompt = prompt.to_owned();
        tokio::task::spawn_blocking(move || -> std::result::Result<Generation, String> {
            use axllm::{AxError, FieldType, ai, ax, tool};
            let key = std::env::var("DEEPSEEK_API_KEY").map_err(|e| e.to_string())?;
            let mut client = ai("deepseek", json!({ "api_key": key, "base_url": "https://api.deepseek.com/v1", "model": model })).map_err(|e| e.to_string())?;
            let mut program = ax("task:string -> answer:string").map_err(|e| e.to_string())?;
            if tools_enabled {
                let reader = host.clone();
                let read = tool("read_file").description("Read src/lib.rs or Cargo.toml from the generated workspace")
                    .arg("path", FieldType::string()).handler(move |args| {
                        let path = args["path"].as_str().unwrap_or("");
                        reader.read_file(path).map(|s| json!({"content":s})).map_err(|e| AxError::runtime(e.to_string()))
                    });
                let writer = host.clone();
                let write = tool("write_file").description("Write complete Rust source to src/lib.rs in the generated workspace")
                    .arg("path", FieldType::string()).arg("content", FieldType::string()).handler(move |args| {
                        let path = args["path"].as_str().unwrap_or("");
                        let content = args["content"].as_str().unwrap_or("");
                        writer.write_file(path, content).map(|s| json!({"result":s})).map_err(|e| AxError::runtime(e.to_string()))
                    });
                let runner = host.clone();
                let run = tool("run_task").description("Run the allowlisted `test` task; return PASS or FAIL and test output")
                    .arg("task", FieldType::string()).handler(move |args| {
                        let task = args["task"].as_str().unwrap_or("");
                        runner.run_task(task).map(|s| json!({"result":s})).map_err(|e| AxError::runtime(e.to_string()))
                    });
                program = program.with_tool(read).with_tool(write).with_tool(run);
            }
            let result = program.forward(&mut client, json!({"task": prompt})).map_err(|e| e.to_string())?;
            let text = result["answer"].as_str().ok_or_else(|| format!("Ax did not return answer: {result}"))?.to_owned();
            // Ax's chat log holds one usage record per model request; absent fields remain unknown.
            let usages: Vec<&Value> = program.get_chat_log().iter().filter_map(|entry| entry.get("usage")).collect();
            let usage_file = if tools_enabled { "usage-agent.json" } else { "usage-plan.json" };
            std::fs::write(host.workspace().parent().unwrap().join(usage_file), serde_json::to_string_pretty(&usages).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            Ok(Generation { text, usage: usage_from_records(&usages) })
        }).await?.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_cached_and_uncached_input() {
        let first = json!({"prompt_tokens": 239, "completion_tokens": 709, "total_tokens": 948});
        let second = json!({"prompt_tokens": 393, "completion_tokens": 80, "cache_read_tokens": 640, "total_tokens": 1113});
        let usage = usage_from_records(&[&first, &second]);
        assert_eq!(usage.input_tokens, Some(1272));
        assert_eq!(usage.output_tokens, Some(789));
        assert_eq!(usage.cached_input_tokens, Some(640));
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
        return Err("tests failed; inspect artifacts".into());
    }
    Ok(())
}
