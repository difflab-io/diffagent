//! Run a named task without an LLM, for external acceptance checks.
use diffagent_core::{spec::LoadedAgent, tools::ToolHost};
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let (Some(agent), Some(workspace), Some(task), None) =
        (args.next(), args.next(), args.next(), args.next())
    else {
        eprintln!("usage: check-task AGENT_YAML WORKSPACE TASK_NAME");
        return ExitCode::from(2);
    };
    match run(Path::new(&agent), Path::new(&workspace), &task) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("task failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(agent: &Path, workspace: &Path, task: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let agent = LoadedAgent::load(agent)?;
    let host = ToolHost::new(workspace, agent.spec.tools)?;
    let output = host.run_named_task(task)?;
    print!("{}", output.output);
    if output.truncated {
        eprintln!("task output truncated after 8 KB");
    }
    Ok(output.success)
}
