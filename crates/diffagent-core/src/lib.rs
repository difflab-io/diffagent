pub mod tools;

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, error::Error, fs, io::Read, path::PathBuf, time::Instant};
use tools::ToolHost;

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
    prompt: Option<String>,
    task: Option<String>,
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

#[derive(Default, Clone, Serialize)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub model_calls: usize,
}

impl Usage {
    fn add(&mut self, other: &Self) {
        if other.model_calls == 0 {
            return;
        }
        fn sum(a: Option<u64>, b: Option<u64>) -> Option<u64> {
            Some(a? + b?)
        }
        self.input_tokens = sum(self.input_tokens, other.input_tokens);
        self.output_tokens = sum(self.output_tokens, other.output_tokens);
        self.cached_input_tokens = sum(self.cached_input_tokens, other.cached_input_tokens);
        self.model_calls += other.model_calls;
    }
}

pub struct Generation {
    pub text: String,
    pub usage: Usage,
}

#[allow(async_fn_in_trait)]
pub trait Backend {
    fn name(&self) -> &'static str;
    async fn generate(
        &self,
        model: &str,
        prompt: &str,
        host: ToolHost,
        tools_enabled: bool,
    ) -> Result<Generation>;
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

pub struct RunArgs {
    pub input: String,
    pub fixture: Option<String>,
    pub run_id: Option<String>,
}

fn safe_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 40
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn args_from_env() -> Result<RunArgs> {
    let mut input = None;
    let mut fixture = None;
    let mut run_id = None;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let value = iter.next().ok_or("every flag needs a value")?;
        match flag.as_str() {
            "--prompt" if input.is_none() => input = Some(value.clone()),
            "--prompt-file" if input.is_none() => {
                let text = if value == "-" {
                    let mut text = String::new();
                    std::io::stdin().read_to_string(&mut text)?;
                    text
                } else {
                    fs::read_to_string(value)?
                };
                input = Some(text);
            }
            "--fixture" if fixture.is_none() && safe_label(value) => fixture = Some(value.clone()),
            "--run-id" if run_id.is_none() && safe_label(value) => run_id = Some(value.clone()),
            _ => return Err(format!("unknown/duplicate/invalid flag: {flag}").into()),
        }
    }
    let input =
        input.ok_or("usage: --prompt TEXT | --prompt-file PATH [--fixture NAME --run-id ID]")?;
    if input.trim().is_empty() {
        return Err("input prompt must not be empty".into());
    }
    if run_id.is_some() && fixture.is_none() {
        return Err("--run-id requires --fixture".into());
    }
    Ok(RunArgs {
        input,
        fixture,
        run_id,
    })
}

fn render(template: &str, input: &str, state: &State) -> String {
    template
        .replace("{{input}}", input)
        .replace("{{plan}}", &state.plan)
        .replace("{{implementation}}", &state.implementation)
        .replace("{{evaluation}}", &state.evaluation)
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

#[derive(Serialize)]
struct Step {
    node: String,
    backend: String,
    model: String,
    elapsed_ms: u128,
    output: String,
    next: String,
    usage: Usage,
    tool_calls: usize,
}

#[derive(Serialize)]
struct Metrics<'a> {
    model: &'a str,
    usage: &'a Usage,
    /// Estimate using reported cache hits (or assuming none if unavailable).
    offpeak_usd_estimate: Option<f64>,
    peak_usd_estimate: Option<f64>,
    cache_tokens_assumed_zero: bool,
    pricing_source: &'static str,
}

pub async fn run(backend: &impl Backend, args: &RunArgs) -> Result<(PathBuf, bool)> {
    let input = &args.input;
    let config_file = config_path()?;
    let root = config_file.parent().ok_or("invalid config path")?;
    let config: Config = serde_yaml::from_str(&fs::read_to_string(&config_file)?)?;
    let (dir, acceptance) = if let Some(fixture) = &args.fixture {
        let acceptance = root
            .join("benchmarks/fixtures")
            .join(fixture)
            .join("tests/acceptance.rs");
        if !acceptance.is_file() {
            return Err(format!("unknown fixture: {fixture}").into());
        }
        (
            root.join("benchmarks/results")
                .join(fixture)
                .join(backend.name())
                .join(args.run_id.as_deref().unwrap_or("1")),
            Some(acceptance),
        )
    } else {
        (root.join("runs").join(backend.name()), None)
    };
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("input.md"), input)?;
    let host = ToolHost::new(dir.join("workspace"), acceptance.as_deref())?;
    let mut state = State::default();
    let mut trace: Vec<Step> = Vec::new();
    let mut total = Usage {
        input_tokens: Some(0),
        output_tokens: Some(0),
        cached_input_tokens: Some(0),
        model_calls: 0,
    };
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
        let filename = if current == "evaluate" && state.updates > 0 {
            "evaluate-after-update"
        } else {
            &current
        };
        let before = host.events().len();
        let start = Instant::now();
        let outcome = if node.task.as_deref() == Some("test") {
            let output = host.run_task("test")?;
            // No implementation or model tool write means this stage cannot pass.
            let wrote = host
                .events()
                .iter()
                .any(|e| e.name == "write_file" && e.success);
            Generation {
                text: if wrote {
                    output
                } else {
                    "FAIL: no write_file tool call occurred".into()
                },
                usage: Usage::default(),
            }
        } else {
            let template = fs::read_to_string(
                root.join(node.prompt.as_deref().ok_or("missing node prompt")?),
            )?;
            let prompt = render(&template, input, &state);
            fs::write(dir.join(format!("{filename}.prompt.md")), &prompt)?;
            backend
                .generate(
                    &config.defaults.model,
                    &prompt,
                    host.clone(),
                    current != "plan",
                )
                .await?
        };
        let elapsed_ms = start.elapsed().as_millis();
        let output = outcome.text;
        fs::write(dir.join(format!("{filename}.md")), &output)?;
        match current.as_str() {
            "plan" => state.plan = output.clone(),
            "implement" | "update" => {
                state.implementation = output.clone();
                if current == "update" {
                    state.updates += 1;
                }
            }
            "evaluate" => {
                state.passed = output.starts_with("PASS:");
                state.evaluation = output.clone();
            }
            other => return Err(format!("unsupported node: {other}").into()),
        }
        let next = route(&current, node, &state, config.graph.max_updates)?;
        total.add(&outcome.usage);
        trace.push(Step {
            node: current.clone(),
            backend: backend.name().into(),
            model: config.defaults.model.clone(),
            elapsed_ms,
            output: format!("{filename}.md"),
            next: next.clone(),
            usage: outcome.usage,
            tool_calls: host.events().len() - before,
        });
        fs::write(
            dir.join("trace.json"),
            serde_json::to_string_pretty(&trace)?,
        )?;
        fs::write(
            dir.join("tools.json"),
            serde_json::to_string_pretty(&host.events())?,
        )?;
        let price = if config.defaults.model == "deepseek-flash" {
            total
                .input_tokens
                .zip(total.output_tokens)
                .and_then(|(input, output)| {
                    let cached = total.cached_input_tokens.unwrap_or(0);
                    let miss = input.checked_sub(cached)?;
                    Some((
                        (miss as f64 * 0.15 + cached as f64 * 0.003 + output as f64 * 0.60)
                            / 1_000_000.0,
                        (miss as f64 * 0.30 + cached as f64 * 0.006 + output as f64 * 1.20)
                            / 1_000_000.0,
                    ))
                })
        } else {
            None
        };
        fs::write(
            dir.join("metrics.json"),
            serde_json::to_string_pretty(&Metrics {
                model: &config.defaults.model,
                usage: &total,
                offpeak_usd_estimate: price.map(|p| p.0),
                peak_usd_estimate: price.map(|p| p.1),
                cache_tokens_assumed_zero: total.cached_input_tokens.is_none(),
                pricing_source: "https://api-docs.deepseek.com/quick_start/pricing/",
            })?,
        )?;
        println!(
            "{current} -> {next}: {} ({} ms, {} tool calls)",
            dir.join(format!("{filename}.md")).display(),
            elapsed_ms,
            host.events().len() - before
        );
        current = next;
    }
    Ok((dir, state.passed))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_routes_to_update_then_stops() {
        let graph: Config = serde_yaml::from_str(include_str!("../../../diffagent.yaml")).unwrap();
        let evaluate = &graph.graph.nodes["evaluate"];
        let state = State::default();
        assert_eq!(route("evaluate", evaluate, &state, 1).unwrap(), "update");
        assert_eq!(
            route("update", &graph.graph.nodes["update"], &state, 1).unwrap(),
            "evaluate"
        );
        assert_eq!(
            route(
                "evaluate",
                evaluate,
                &State {
                    updates: 1,
                    ..State::default()
                },
                1
            )
            .unwrap(),
            "end"
        );
    }
}
