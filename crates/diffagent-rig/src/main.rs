use diffagent_core::{Backend, Result};

struct Rig;

impl Backend for Rig {
    fn name(&self) -> &'static str {
        "rig"
    }

    async fn generate(&self, model: &str, prompt: &str) -> Result<String> {
        use rig::agent::AgentBuilder;
        let client = rig::providers::deepseek::from_env()?;
        let agent = AgentBuilder::new(client.chat(model)).build();
        Ok(agent.prompt(prompt).await?.output)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let input = diffagent_core::input_from_args()?;
    let (dir, passed) = diffagent_core::run(&Rig, &input).await?;
    println!(
        "Rig: {} | artifacts: {}",
        if passed { "PASS" } else { "FAIL" },
        dir.display()
    );
    if !passed {
        return Err("evaluation failed; inspect artifacts".into());
    }
    Ok(())
}
