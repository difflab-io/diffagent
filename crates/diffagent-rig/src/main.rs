use diffagent_core::{Backend, Generation, Result, Usage, tools::ToolHost};
use rig::{
    agent::AgentBuilder,
    tool::{Tool, ToolContext},
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct PathArg {
    path: String,
}
#[derive(Deserialize)]
struct WriteArg {
    path: String,
    content: String,
}
#[derive(Deserialize)]
struct TaskArg {
    task: String,
}

#[derive(Clone)]
struct Read(ToolHost);
impl Tool for Read {
    const NAME: &'static str = "read_file";
    type Args = PathArg;
    type Output = String;
    type Error = std::io::Error;
    fn description(&self) -> String {
        "Read src/lib.rs or Cargo.toml in the generated workspace".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    async fn call(&self, _: &mut ToolContext, args: PathArg) -> std::io::Result<String> {
        self.0.read_file(&args.path)
    }
}
#[derive(Clone)]
struct Write(ToolHost);
impl Tool for Write {
    const NAME: &'static str = "write_file";
    type Args = WriteArg;
    type Output = String;
    type Error = std::io::Error;
    fn description(&self) -> String {
        "Write complete Rust source to src/lib.rs in the generated workspace".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]})
    }
    async fn call(&self, _: &mut ToolContext, args: WriteArg) -> std::io::Result<String> {
        self.0.write_file(&args.path, &args.content)
    }
}
#[derive(Clone)]
struct Task(ToolHost);
impl Tool for Task {
    const NAME: &'static str = "run_task";
    type Args = TaskArg;
    type Output = String;
    type Error = std::io::Error;
    fn description(&self) -> String {
        "Run the allowlisted `test` task in the generated Rust workspace; returns PASS or FAIL and compiler/test output".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"task":{"type":"string","enum":["test"]}},"required":["task"]})
    }
    async fn call(&self, _: &mut ToolContext, args: TaskArg) -> std::io::Result<String> {
        self.0.run_task(&args.task)
    }
}

struct Rig;
impl Backend for Rig {
    fn name(&self) -> &'static str {
        "rig"
    }
    async fn generate(
        &self,
        model: &str,
        prompt: &str,
        host: ToolHost,
        tools_enabled: bool,
    ) -> Result<Generation> {
        let client = rig::providers::deepseek::from_env()?;
        let builder = AgentBuilder::new(client.chat(model));
        let agent = if tools_enabled {
            builder
                .tool(Read(host.clone()))
                .tool(Write(host.clone()))
                .tool(Task(host))
                .build()
        } else {
            builder.build()
        };
        let response = agent.prompt(prompt).max_turns(10).run().await?;
        Ok(Generation {
            text: response.output,
            usage: Usage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
                cached_input_tokens: response.usage.cached_input_tokens,
                model_calls: response.completion_calls.len(),
            },
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = diffagent_core::args_from_env()?;
    let (dir, passed) = diffagent_core::run(&Rig, &args).await?;
    println!(
        "Rig: {} | artifacts: {}",
        if passed { "PASS" } else { "FAIL" },
        dir.display()
    );
    if !passed {
        return Err("tests failed; inspect artifacts".into());
    }
    Ok(())
}
