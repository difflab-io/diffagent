//! Shared, SDK-independent YAML declarations and safe text interpolation.
//!
//! This crate parses configuration only. Ax, ADK, and Rig own their own agent
//! loops and flow executors. In particular, there is no backend trait here.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, SpecError>;

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSpec {
    pub version: u32,
    pub name: String,
    pub model: String,
    /// Path to the system prompt Markdown file, relative to the YAML file.
    pub system_prompt: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub flows: Vec<String>,
    #[serde(default)]
    pub tools: ToolSpec,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolSpec {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
    /// Allow agents to list and run mise tasks found in the workspace.
    #[serde(default)]
    pub mise_tasks: bool,
    /// A named task is an argv vector, never a shell command string.
    #[serde(default)]
    pub tasks: BTreeMap<String, TaskSpec>,
    #[serde(default = "default_max_write_bytes")]
    pub max_write_bytes: usize,
}

const fn default_max_write_bytes() -> usize {
    256 * 1024
}

impl Default for ToolSpec {
    fn default() -> Self {
        Self {
            read: Vec::new(),
            write: Vec::new(),
            mise_tasks: false,
            tasks: BTreeMap::new(),
            max_write_bytes: default_max_write_bytes(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub argv: Vec<String>,
    #[serde(default = "default_task_timeout")]
    pub timeout_secs: u64,
}

const fn default_task_timeout() -> u64 {
    45
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlowSpec {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterSpec>,
    pub start: String,
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    #[serde(default)]
    pub questions: QuestionPolicy,
    pub nodes: BTreeMap<String, FlowNode>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionPolicy {
    #[default]
    PromptWhenInteractive,
    Skip,
}

const fn default_max_steps() -> usize {
    24
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterSpec {
    #[serde(rename = "type")]
    pub kind: ParameterType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FlowNode {
    Agent {
        /// Markdown prompt template, relative to the flow YAML file.
        prompt: String,
        model: String,
        next: String,
    },
    Task {
        /// A key in AgentSpec.tools.tasks.
        task: String,
        on_pass: String,
        on_fail: String,
    },
    End,
}

#[derive(Clone, Debug)]
pub struct Skill {
    pub name: String,
    pub instructions: String,
}

#[derive(Clone, Debug)]
pub struct LoadedFlow {
    pub spec: FlowSpec,
    pub root: PathBuf,
}

impl LoadedFlow {
    pub fn prompt(&self, node: &str) -> Result<String> {
        match self.spec.nodes.get(node) {
            Some(FlowNode::Agent { prompt, .. }) => read_relative(&self.root, prompt),
            _ => Err(SpecError::Invalid(format!("{node} is not an agent node"))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoadedAgent {
    pub spec: AgentSpec,
    pub system_prompt: String,
    pub skills: Vec<Skill>,
    pub flows: BTreeMap<String, LoadedFlow>,
    pub root: PathBuf,
}

impl LoadedAgent {
    pub fn load(path: &Path) -> Result<Self> {
        let path = path.canonicalize()?;
        let root = path
            .parent()
            .ok_or_else(|| SpecError::Invalid("agent YAML has no parent".into()))?
            .to_path_buf();
        let spec: AgentSpec = serde_yaml::from_str(&read_bounded(&path)?)?;
        spec.validate()?;
        let system_prompt = read_relative(&root, &spec.system_prompt)?;
        let mut skills = Vec::new();
        for skill in &spec.skills {
            let path = if skill.ends_with(".md") {
                skill.clone()
            } else {
                format!("{skill}/SKILL.md")
            };
            skills.push(Skill {
                name: skill.clone(),
                instructions: read_relative(&root, &path)?,
            });
        }
        let mut flows = BTreeMap::new();
        for flow_path in &spec.flows {
            let absolute = resolve_relative(&root, flow_path)?;
            let flow: FlowSpec = serde_yaml::from_str(&read_bounded(&absolute)?)?;
            flow.validate()?;
            let flow_root = absolute
                .parent()
                .ok_or_else(|| SpecError::Invalid("flow has no parent".into()))?
                .to_path_buf();
            for node in flow.nodes.values() {
                if let FlowNode::Agent { prompt, .. } = node {
                    read_relative(&flow_root, prompt)?;
                }
                if let FlowNode::Task { task, .. } = node
                    && !spec.tools.tasks.contains_key(task)
                {
                    return Err(SpecError::Invalid(format!(
                        "flow {} uses unconfigured task {task}",
                        flow.name
                    )));
                }
            }
            let name = flow.name.clone();
            if flows
                .insert(
                    name.clone(),
                    LoadedFlow {
                        spec: flow,
                        root: flow_root,
                    },
                )
                .is_some()
            {
                return Err(SpecError::Invalid(format!("duplicate flow name: {name}")));
            }
        }
        Ok(Self {
            spec,
            system_prompt,
            skills,
            flows,
            root,
        })
    }
}

impl AgentSpec {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || self.name.trim().is_empty() || self.model.trim().is_empty() {
            return Err(SpecError::Invalid(
                "agent needs version: 1, a name, and a model".into(),
            ));
        }
        if self.tools.max_write_bytes == 0 || self.tools.max_write_bytes > 1024 * 1024 {
            return Err(SpecError::Invalid(
                "max_write_bytes must be 1..=1048576".into(),
            ));
        }
        for (name, task) in &self.tools.tasks {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                return Err(SpecError::Invalid(format!("invalid task name: {name}")));
            }
            if task.argv.is_empty()
                || task.argv[0].is_empty()
                || task.timeout_secs == 0
                || task.timeout_secs > 300
            {
                return Err(SpecError::Invalid(format!(
                    "task {name} needs argv and timeout_secs 1..=300"
                )));
            }
        }
        Ok(())
    }
}

impl FlowSpec {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.name.trim().is_empty()
            || self.max_steps == 0
            || self.max_steps > 128
        {
            return Err(SpecError::Invalid(
                "flow needs version: 1, a name, and max_steps 1..=128".into(),
            ));
        }
        self.check_target(&self.start)?;
        for (name, node) in &self.nodes {
            match node {
                FlowNode::Agent {
                    next,
                    prompt,
                    model,
                } => {
                    if prompt.is_empty() || model.trim().is_empty() {
                        return Err(SpecError::Invalid(format!(
                            "{name}: missing prompt or model"
                        )));
                    }
                    self.check_target(next)?;
                }
                FlowNode::Task {
                    on_pass, on_fail, ..
                } => {
                    self.check_target(on_pass)?;
                    self.check_target(on_fail)?;
                }
                FlowNode::End => {}
            }
        }
        for (name, param) in &self.parameters {
            if name.is_empty()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || (param.required && param.default.is_some())
            {
                return Err(SpecError::Invalid(format!(
                    "invalid flow parameter: {name}"
                )));
            }
        }
        Ok(())
    }

    fn check_target(&self, target: &str) -> Result<()> {
        if target != "end" && !self.nodes.contains_key(target) {
            return Err(SpecError::Invalid(format!(
                "flow {} has unknown node: {target}",
                self.name
            )));
        }
        Ok(())
    }

    pub fn arguments(
        &self,
        provided: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, String>> {
        let mut arguments = BTreeMap::new();
        for name in provided.keys() {
            if !self.parameters.contains_key(name) {
                return Err(SpecError::Invalid(format!("unknown flow argument: {name}")));
            }
        }
        for (name, parameter) in &self.parameters {
            let value = provided.get(name).or(parameter.default.as_ref());
            match value {
                Some(value) if value.len() <= 32_000 => {
                    arguments.insert(name.clone(), value.clone());
                }
                Some(_) => {
                    return Err(SpecError::Invalid(format!(
                        "flow argument {name} exceeds 32 KB"
                    )));
                }
                None if parameter.required => {
                    return Err(SpecError::Invalid(format!("missing flow argument: {name}")));
                }
                None => {}
            }
        }
        Ok(arguments)
    }
}

#[derive(Clone, Debug, Default)]
pub struct StepValue {
    pub output: String,
    pub passed: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct FlowValues {
    pub parameters: BTreeMap<String, String>,
    pub steps: BTreeMap<String, StepValue>,
}

impl FlowValues {
    /// Only named parameter and completed-node values are interpolated. No code is evaluated.
    pub fn render(&self, template: &str) -> Result<String> {
        let mut rest = template;
        let mut result = String::new();
        while let Some(start) = rest.find("{{") {
            result.push_str(&rest[..start]);
            rest = &rest[start + 2..];
            let end = rest
                .find("}}")
                .ok_or_else(|| SpecError::Invalid("unclosed {{ in template".into()))?;
            let key = rest[..end].trim();
            let value = if let Some(name) = key.strip_prefix("params.") {
                self.parameters
                    .get(name)
                    .ok_or_else(|| SpecError::Invalid(format!("unknown parameter: {name}")))?
                    .clone()
            } else if let Some(path) = key.strip_prefix("steps.") {
                let (name, field) = path
                    .rsplit_once('.')
                    .ok_or_else(|| SpecError::Invalid(format!("invalid step reference: {key}")))?;
                let step = self
                    .steps
                    .get(name)
                    .ok_or_else(|| SpecError::Invalid(format!("step {name} is not complete")))?;
                match field {
                    "output" => step.output.clone(),
                    "passed" => step
                        .passed
                        .map(|passed| passed.to_string())
                        .ok_or_else(|| {
                            SpecError::Invalid(format!("step {name} has no pass result"))
                        })?,
                    _ => return Err(SpecError::Invalid(format!("invalid step field: {field}"))),
                }
            } else {
                return Err(SpecError::Invalid(format!(
                    "invalid template expression: {key}"
                )));
            };
            result.push_str(&value);
            rest = &rest[end + 2..];
        }
        result.push_str(rest);
        Ok(result)
    }
}

const MAX_DOCUMENT: u64 = 128 * 1024;

fn read_bounded(path: &Path) -> Result<String> {
    if path.metadata()?.len() > MAX_DOCUMENT {
        return Err(SpecError::Invalid(format!(
            "document exceeds 128 KB: {}",
            path.display()
        )));
    }
    Ok(fs::read_to_string(path)?)
}

fn resolve_relative(root: &Path, relative: &str) -> Result<PathBuf> {
    let requested = Path::new(relative);
    if requested.as_os_str().is_empty()
        || !requested
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(SpecError::Invalid(format!(
            "only relative paths without traversal are allowed: {relative}"
        )));
    }
    let root = root.canonicalize()?;
    let resolved = root.join(requested).canonicalize()?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
        return Err(SpecError::Invalid(format!(
            "document must stay in configuration directory: {relative}"
        )));
    }
    Ok(resolved)
}

fn read_relative(root: &Path, relative: &str) -> Result<String> {
    read_bounded(&resolve_relative(root, relative)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_tools_have_a_valid_default_limit() {
        let agent: AgentSpec = serde_yaml::from_str(
            "version: 1\nname: chat\nmodel: deepseek-flash\nsystem_prompt: SYSTEM.md\n",
        )
        .unwrap();
        agent.validate().unwrap();
        assert_eq!(agent.tools.max_write_bytes, 256 * 1024);
    }

    #[test]
    fn flow_question_policy_defaults_and_rejects_unknown_values() {
        let yaml = "version: 1\nname: sample\nstart: end\nnodes: {}\n";
        let default: FlowSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(default.questions, QuestionPolicy::PromptWhenInteractive);
        let skipped: FlowSpec =
            serde_yaml::from_str(&yaml.replace("nodes:", "questions: skip\nnodes:")).unwrap();
        assert_eq!(skipped.questions, QuestionPolicy::Skip);
        assert!(
            serde_yaml::from_str::<FlowSpec>(&yaml.replace("nodes:", "questions: always\nnodes:"))
                .is_err()
        );
    }

    #[test]
    fn interpolates_only_known_inputs() {
        let context = FlowValues {
            parameters: BTreeMap::from([("request".into(), "build a game".into())]),
            steps: BTreeMap::from([(
                "plan".into(),
                StepValue {
                    output: "add a board".into(),
                    passed: None,
                },
            )]),
        };
        assert_eq!(
            context
                .render("{{ params.request }}: {{ steps.plan.output }}")
                .unwrap(),
            "build a game: add a board"
        );
        assert!(context.render("{{ steps.missing.output }}").is_err());
        assert!(context.render("{{ std::process::exit(0) }}").is_err());
        assert!(context.render("{{ params.request").is_err());
    }

    #[test]
    fn validates_graph_targets_and_parameters() {
        let flow: FlowSpec = serde_yaml::from_str("version: 1\nname: sample\nstart: plan\nparameters:\n  request: {type: string, required: true}\nnodes:\n  plan: {kind: agent, prompt: plan.md, model: deepseek-flash, next: end}\n").unwrap();
        flow.validate().unwrap();
        assert!(flow.arguments(&BTreeMap::new()).is_err());
        assert!(
            flow.arguments(&BTreeMap::from([("request".into(), "hello".into())]))
                .is_ok()
        );
        let broken: FlowSpec =
            serde_yaml::from_str("version: 1\nname: broken\nstart: missing\nnodes: {}\n").unwrap();
        assert!(broken.validate().is_err());
    }

    #[test]
    fn loads_prompt_skills_and_flow_inside_config_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("skills/demo")).unwrap();
        fs::create_dir_all(dir.path().join("flows")).unwrap();
        fs::write(dir.path().join("SYSTEM.md"), "Be helpful").unwrap();
        fs::write(dir.path().join("skills/demo/SKILL.md"), "# Demo skill").unwrap();
        fs::write(dir.path().join("flows/plan.md"), "{{ params.request }}").unwrap();
        fs::write(dir.path().join("flows/sample.yaml"), "version: 1\nname: sample\nstart: plan\nparameters:\n  request: {type: string, required: true}\nnodes:\n  plan: {kind: agent, prompt: plan.md, model: deepseek-flash, next: end}\n").unwrap();
        fs::write(dir.path().join("agent.yaml"), "version: 1\nname: example\nmodel: deepseek-flash\nsystem_prompt: SYSTEM.md\nskills: [skills/demo]\nflows: [flows/sample.yaml]\ntools:\n  read: [src/**]\n  write: [src/**]\n  tasks:\n    test: {argv: [cargo, test, --offline], timeout_secs: 45}\n").unwrap();
        let loaded = LoadedAgent::load(&dir.path().join("agent.yaml")).unwrap();
        assert_eq!(loaded.system_prompt, "Be helpful");
        assert_eq!(loaded.skills[0].instructions, "# Demo skill");
        assert_eq!(
            loaded.flows["sample"].prompt("plan").unwrap(),
            "{{ params.request }}"
        );
    }

    #[test]
    fn repository_example_is_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let agent = LoadedAgent::load(&root.join("agent.yaml")).unwrap();
        assert_eq!(agent.flows["implement"].spec.max_steps, 12);
        let benchmark = LoadedAgent::load(&root.join("agent-benchmark.yaml")).unwrap();
        assert!(
            !benchmark
                .spec
                .tools
                .read
                .iter()
                .any(|path| path.starts_with("tests"))
        );
        assert!(
            agent.flows["implement"]
                .prompt("implement")
                .unwrap()
                .contains("steps.plan.output")
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_references_outside_root() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        symlink(outside.path(), dir.path().join("escape.md")).unwrap();
        assert!(resolve_relative(dir.path(), "escape.md").is_err());
        assert!(resolve_relative(dir.path(), "../escape.md").is_err());
    }
}
