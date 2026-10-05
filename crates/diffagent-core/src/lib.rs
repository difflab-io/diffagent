use std::{collections::BTreeMap, env, error::Error, fs, io::Read, path::PathBuf, time::Instant};

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
struct Config {
    defaults: Defaults,
    graph: Graph,
}

#[derive(Deserialize)]
struct Defaults {
    model: String,
}

#[derive(Deserialize)]
struct Graph {
    start: String,
    max_updates: usize,
    nodes: BTreeMap<String, Node>,
}

#[derive(Deserialize)]
struct Node {
    prompt: String,
    next: Option<String>,
    on_pass: Option<String>,
    on_fail: Option<String>,
}

#[derive(Default)]
struct State {
    plan: String,
    implementation: String,
    evaluation: String,
    passed: bool,
    updates: usize,
}

#[derive(Serialize)]
struct Step {
    node: String,
    backend: String,
    model: String,
    elapsed_ms: u128,
    output: String,
    next: String,
}

#[allow(async_fn_in_trait)] // This POC uses only statically dispatched, in-process backends.
pub trait Backend {
    fn name(&self) -> &'static str;
    async fn generate(&self, model: &str, prompt: &str) -> Result<String>;
}

fn config_path() -> Result<PathBuf> {
    for dir in env::current_dir()?.ancestors() {
        let path = dir.join("diffagent.yaml");
        if path.is_file() {
            return Ok(path);
        }
    }
    let path = PathBuf::from(env::var("HOME")?).join(".difflab/diffagent.yaml");
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("diffagent.yaml not found; also checked {}", path.display()).into())
    }
}

/// Both CLIs accept an input prompt at runtime; the graph never hardcodes the request.
pub fn input_from_args() -> Result<String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input = match args.as_slice() {
        [flag, value] if flag == "--prompt" => value.clone(),
        [flag, value] if flag == "--prompt-file" && value == "-" => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            input
        }
        [flag, value] if flag == "--prompt-file" => fs::read_to_string(value)?,
        _ => {
            return Err(
                "usage: --prompt 'your request' | --prompt-file path (use - for stdin)".into(),
            );
        }
    };
    if input.trim().is_empty() {
        return Err("input prompt must not be empty".into());
    }
    Ok(input)
}

fn render(template: &str, input: &str, state: &State) -> String {
    template
        .replace("{{input}}", input)
        .replace("{{plan}}", &state.plan)
        .replace("{{implementation}}", &state.implementation)
        .replace("{{evaluation}}", &state.evaluation)
}

fn rust_code(output: &str) -> Option<&str> {
    let body = output.strip_prefix("```rust\n")?;
    body.split_once("\n```").map(|(code, _)| code)
}

fn route(node_id: &str, node: &Node, state: &State, max_updates: usize) -> Result<String> {
    let next = if node_id == "evaluate" {
        if state.passed {
            node.on_pass.as_deref()
        } else if state.updates >= max_updates {
            Some("end")
        } else {
            node.on_fail.as_deref()
        }
    } else {
        node.next.as_deref()
    };
    next.map(str::to_owned)
        .ok_or_else(|| format!("missing outgoing edge: {node_id}").into())
}

/// Runs the same graph for either CLI and saves prompts, outputs, and trace for comparison.
pub async fn run(backend: &impl Backend, input: &str) -> Result<(PathBuf, bool)> {
    let config_file = config_path()?;
    let root = config_file.parent().ok_or("invalid config path")?;
    let config: Config = serde_yaml::from_str(&fs::read_to_string(&config_file)?)?;
    let dir = root.join("runs").join(backend.name());
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    } // Only our fixed backend-named artifact directory.
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("input.md"), input)?;
    let mut state = State::default();
    let mut trace = Vec::new();
    let mut current = config.graph.start.clone();
    let max_steps = config
        .graph
        .nodes
        .len()
        .saturating_mul(config.graph.max_updates.saturating_add(2));
    while current != "end" {
        if trace.len() >= max_steps {
            return Err("graph exceeded its step limit".into());
        }
        let node = config
            .graph
            .nodes
            .get(&current)
            .ok_or_else(|| format!("unknown node: {current}"))?;
        let template = fs::read_to_string(root.join(&node.prompt))?;
        let prompt = render(&template, input, &state);
        let filename = if current == "evaluate" && state.updates > 0 {
            "evaluate-after-update"
        } else {
            &current
        };
        fs::write(dir.join(format!("{filename}.prompt.md")), &prompt)?;
        let start = Instant::now();
        let output = backend.generate(&config.defaults.model, &prompt).await?;
        let elapsed_ms = start.elapsed().as_millis();
        fs::write(dir.join(format!("{filename}.md")), &output)?;
        if (current == "implement" || current == "update")
            && let Some(code) = rust_code(&output)
        {
            fs::write(dir.join(format!("{filename}.rs")), code)?;
        }
        match current.as_str() {
            "plan" => state.plan = output.clone(),
            "implement" | "update" => {
                state.implementation = output.clone();
                if current == "update" {
                    state.updates += 1;
                }
            }
            "evaluate" => {
                state.passed = output
                    .lines()
                    .next()
                    .is_some_and(|line| line.trim() == "PASS");
                state.evaluation = output.clone();
            }
            other => return Err(format!("unsupported node: {other}").into()),
        }
        let next = route(&current, node, &state, config.graph.max_updates)?;
        trace.push(Step {
            node: current.clone(),
            backend: backend.name().into(),
            model: config.defaults.model.clone(),
            elapsed_ms,
            output: format!("{filename}.md"),
            next: next.clone(),
        });
        fs::write(
            dir.join("trace.json"),
            serde_json::to_string_pretty(&trace)?,
        )?;
        println!(
            "{current} -> {next} ({elapsed_ms} ms): {}",
            dir.join(format!("{filename}.md")).display()
        );
        current = next;
    }
    Ok((dir, state.passed))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_routes_to_update_and_then_stops() {
        let graph: Config = serde_yaml::from_str(include_str!("../../../diffagent.yaml")).unwrap();
        let evaluate = &graph.graph.nodes["evaluate"];
        let state = State::default();
        assert_eq!(route("evaluate", evaluate, &state, 1).unwrap(), "update");
        assert_eq!(
            route("update", &graph.graph.nodes["update"], &state, 1).unwrap(),
            "evaluate"
        );
        let done = State {
            updates: 1,
            ..State::default()
        };
        assert_eq!(route("evaluate", evaluate, &done, 1).unwrap(), "end");
        let passed = State {
            passed: true,
            ..State::default()
        };
        assert_eq!(route("evaluate", evaluate, &passed, 1).unwrap(), "end");
    }

    #[test]
    fn extracts_rust_without_executing_it() {
        assert_eq!(
            rust_code("```rust\nfn main() {}\n```"),
            Some("fn main() {}")
        );
    }

    #[test]
    fn input_is_rendered_without_a_fixed_task() {
        let state = State::default();
        assert_eq!(
            render("Request: {{input}}", "build a calculator", &state),
            "Request: build a calculator"
        );
    }
}
