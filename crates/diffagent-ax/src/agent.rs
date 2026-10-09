use anyhow::{Context, Result};
use axllm::runtime::quickjs::QuickJsCodeRuntime;
use axllm::{AxAgent, AxGen, agent_with_options, ai, ax};
use diffagent_core::events::{ActivityGuard, Event, QuestionAnswer, QuestionHandler};
use diffagent_core::spec::LoadedAgent;
use diffagent_core::tools::ToolHost;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, mpsc::Sender},
};

// Types and structs -----------------------------------------------------------
#[derive(Clone)]
pub struct Engine {
    pub(crate) agent: Arc<LoadedAgent>,
    pub(crate) host: ToolHost,
    pub(crate) events: Sender<Event>,
    pub(crate) question_handler: QuestionHandler,
}

// Ax keeps workflow nodes and chat on different native programs. Do not erase
// this distinction behind a shared executor: their tool and runtime APIs differ.
enum DefinedAgent {
    Flow(AxGen),
    Chat(AxAgent),
}

// Public API ------------------------------------------------------------------
impl Engine {
    pub fn load(config: &Path, workspace: &Path, events: Sender<Event>) -> Result<Self> {
        let agent = LoadedAgent::load(config)?;
        let host = ToolHost::new(workspace, agent.spec.tools.clone())?;
        Ok(Self {
            agent: Arc::new(agent),
            host,
            events,
            question_handler: Arc::new(|_, _| QuestionAnswer::Skipped {
                reason: "question UI unavailable".into(),
            }),
        })
    }

    pub fn with_question_handler(mut self, handler: QuestionHandler) -> Self {
        self.question_handler = handler;
        self
    }
}

// Helpers ---------------------------------------------------------------------
impl Engine {
    pub(crate) fn emit(&self, event: Event) {
        let _ = self.events.send(event);
    }

    pub(crate) fn instructions(&self) -> String {
        let mut text = self.agent.system_prompt.clone();
        for skill in &self.agent.skills {
            text.push_str(&format!(
                "\n\nSkill {}:\n{}",
                skill.name, skill.instructions
            ));
        }
        text
    }

    fn define_agent(&self, flow_node: bool, allow_questions: bool) -> Result<DefinedAgent> {
        if flow_node {
            // Workflow nodes use native provider tool calls, not a JS actor.
            let mut program = ax("task:string -> answer:string")?;
            for host_tool in self.define_tools(true, allow_questions) {
                program = program.with_tool(host_tool);
            }
            Ok(DefinedAgent::Flow(program))
        } else {
            // Chat uses AxAgent's QuickJS actor and a workspace tool module.
            let program = agent_with_options(
                "task:string -> answer:string",
                json!({"name":"diffagent", "directResponse":"off", "maxSteps":32}),
            )?
            .with_runtime(Box::new(QuickJsCodeRuntime::new()))?
            .with_tool_module("workspace", self.define_tools(false, allow_questions))?;
            Ok(DefinedAgent::Chat(program))
        }
    }

    pub(crate) fn generate(
        &self,
        model: &str,
        prompt: &str,
        flow_node: bool,
        allow_questions: bool,
    ) -> Result<String> {
        let _activity = ActivityGuard::new(&self.events, format!("Thinking with {model}..."));
        let key = std::env::var("DEEPSEEK_API_KEY").context("DEEPSEEK_API_KEY is required")?;
        let mise_guidance = match (self.agent.spec.tools.mise_tasks, flow_node) {
            (true, true) => {
                "Use list_mise_tasks to find available mise tasks, then run_mise_task to test a relevant change when the flow's test node does not already cover it."
            }
            (true, false) => {
                "If no configured task fits, call await workspace.list_mise_tasks({}) to find mise tasks, then await workspace.run_mise_task({name:'NAME'}) to run one."
            }
            (false, _) => "",
        };
        let prompt = if flow_node {
            format!(
                "{prompt}\n\nUse the available workspace tools to inspect or edit files. {mise_guidance} The flow graph also runs its configured test task in a later node."
            )
        } else {
            let tasks = self
                .agent
                .spec
                .tools
                .tasks
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{prompt}\n\nWorkspace tools are host functions, not local JavaScript file APIs. Call them with objects, for example await workspace.read_file({{path:'src/lib.rs'}}) and await workspace.write_file({{path:'src/lib.rs',content:source}}). Available task names: {tasks}. When asked to run one, call await workspace.run_task({{task:'NAME'}}) with an actual configured name. {mise_guidance} When producing JavaScript actor code, make sure it is a valid JSON string: no Markdown fences, XML tags, or backticks inside the generated code. After using tools, return the required answer field as one JSON object."
            )
        };
        let mut client = ai(
            "deepseek",
            json!({"api_key":key, "base_url":"https://api.deepseek.com/v1", "model":model}),
        )?;
        match self.define_agent(flow_node, allow_questions)? {
            DefinedAgent::Flow(mut program) => {
                let events = self.events.clone();
                let result = program.streaming_forward(
                    &mut client,
                    json!({"task":prompt}),
                    json!({"maxSteps":16}),
                    move |update| {
                        if let Some(delta) = update.delta.get("answer").and_then(Value::as_str) {
                            let _ = events.send(Event::TextDelta { text: delta.into() });
                        }
                        Ok(())
                    },
                )?;
                Ok(result["answer"]
                    .as_str()
                    .context("AxGen did not return an answer")?
                    .to_owned())
            }
            DefinedAgent::Chat(mut program) => {
                // Ax streams the responder only after the actor has used its tools.
                // No tool-argument token stream is implied by these deltas.
                let events = self.events.clone();
                let mut version = None;
                let result = program.streaming_forward(
                    &mut client,
                    json!({"task":prompt}),
                    json!({}),
                    move |update| {
                        if let Some(delta) = update.delta.get("answer").and_then(Value::as_str) {
                            if version != Some(update.version) {
                                version = Some(update.version);
                            }
                            let _ = events.send(Event::TextDelta { text: delta.into() });
                        }
                        Ok(())
                    },
                )?;
                Ok(result["answer"]
                    .as_str()
                    .context("Ax did not return an answer")?
                    .to_owned())
            }
        }
    }
}
