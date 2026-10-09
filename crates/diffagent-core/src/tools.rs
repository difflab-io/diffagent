//! Framework-neutral, workspace-scoped tool policy and side effects.
//! A preview is a complete proposed write, not a simulated stream of tokens.

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::spec::ToolSpec;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Serialize;
use tempfile::NamedTempFile;
use wait_timeout::ChildExt;

const MAX_READ_BYTES: u64 = 32 * 1024;
const MAX_OUTPUT_BYTES: usize = 8 * 1024;
const MAX_EVENTS: usize = 2048;

pub type Result<T> = std::result::Result<T, ToolError>;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("denied: {0}")]
    Denied(String),
    #[error("invalid policy: {0}")]
    Policy(String),
    #[error("preview rejected: {0}")]
    Preview(String),
    #[error("task execution requires macOS sandbox-exec")]
    SandboxUnavailable,
    #[error("task timed out after {0}s")]
    Timeout(u64),
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAction {
    Read,
    Write,
    List,
    Task,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolEvent {
    pub action: ToolAction,
    pub argument: String,
    pub success: bool,
    pub summary: String,
    pub elapsed_ms: u128,
}

pub struct WriteProposal<'a> {
    pub relative: &'a str,
    pub original: Option<&'a str>,
    pub content: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WriteReceipt {
    pub relative: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub output: String,
    pub truncated: bool,
}

#[derive(Clone)]
pub struct ToolHost {
    workspace: PathBuf,
    policy: ToolSpec,
    read: GlobSet,
    write: GlobSet,
    events: Arc<Mutex<VecDeque<ToolEvent>>>,
}

impl ToolHost {
    pub fn new(workspace: &Path, policy: ToolSpec) -> Result<Self> {
        if !workspace.is_dir() {
            return Err(ToolError::Policy(
                "workspace must exist as a directory".into(),
            ));
        }
        let workspace = workspace.canonicalize()?;
        if policy.max_write_bytes == 0 || policy.max_write_bytes > 1024 * 1024 {
            return Err(ToolError::Policy(
                "max_write_bytes must be 1..=1048576".into(),
            ));
        }
        for (name, task) in &policy.tasks {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || task.argv.is_empty()
                || task.argv.iter().any(|s| s.is_empty() || s.contains('\0'))
                || !(1..=300).contains(&task.timeout_secs)
                || is_shell(&task.argv[0])
                || Path::new(&task.argv[0])
                    .components()
                    .any(|c| matches!(c, Component::ParentDir))
            {
                return Err(ToolError::Policy(format!("invalid task: {name}")));
            }
        }
        Ok(Self {
            workspace,
            read: compile_globs(&policy.read)?,
            write: compile_globs(&policy.write)?,
            policy,
            events: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
    pub fn events(&self) -> Vec<ToolEvent> {
        self.events.lock().unwrap().iter().cloned().collect()
    }

    fn record<T>(
        &self,
        action: ToolAction,
        argument: &str,
        start: Instant,
        result: &Result<T>,
        summary: &str,
    ) {
        let event = ToolEvent {
            action,
            argument: argument.chars().take(256).collect(),
            success: result.is_ok(),
            summary: if result.is_ok() {
                summary.into()
            } else {
                result
                    .as_ref()
                    .err()
                    .unwrap()
                    .to_string()
                    .chars()
                    .take(512)
                    .collect()
            },
            elapsed_ms: start.elapsed().as_millis(),
        };
        let mut events = self.events.lock().unwrap();
        if events.len() == MAX_EVENTS {
            events.pop_front();
        }
        events.push_back(event);
    }

    /// Resolve an allowed path, checking *every* existing component for symlinks.
    /// No implicit directory creation: the parent must already exist.
    fn path(&self, relative: &str, writable: bool) -> Result<PathBuf> {
        let rel = Path::new(relative);
        if rel.as_os_str().is_empty()
            || relative
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || rel.components().any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(ToolError::Denied(
                "expected a relative path without traversal".into(),
            ));
        }
        if !if writable { &self.write } else { &self.read }.is_match(relative) {
            return Err(ToolError::Denied(format!(
                "path not allowlisted: {relative}"
            )));
        }
        let mut path = self.workspace.clone();
        let count = rel.components().count();
        for (index, component) in rel.components().enumerate() {
            path.push(component);
            match fs::symlink_metadata(&path) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() {
                        return Err(ToolError::Denied("symlink traversal is forbidden".into()));
                    }
                    if index + 1 < count && !meta.is_dir() {
                        return Err(ToolError::Denied("parent is not a directory".into()));
                    }
                }
                Err(err)
                    if err.kind() == io::ErrorKind::NotFound && writable && index + 1 == count => {}
                Err(err) => return Err(err.into()),
            }
        }
        Ok(path)
    }

    pub fn read_file(&self, relative: &str) -> Result<String> {
        let start = Instant::now();
        let result = (|| {
            let path = self.path(relative, false)?;
            let mut bytes = Vec::new();
            File::open(path)?
                .take(MAX_READ_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_READ_BYTES {
                return Err(ToolError::Denied("file exceeds read limit".into()));
            }
            String::from_utf8(bytes).map_err(|_| ToolError::Denied("file is not UTF-8".into()))
        })();
        self.record(ToolAction::Read, relative, start, &result, "read file");
        result
    }

    /// Commit only after the complete proposed content is approved. A rejected
    /// preview, failed write, or failed rename leaves the previous file intact.
    pub fn write_file<F>(&self, relative: &str, content: &str, preview: F) -> Result<WriteReceipt>
    where
        F: FnOnce(&WriteProposal<'_>) -> Result<()>,
    {
        let start = Instant::now();
        let result = (|| {
            if content.len() > self.policy.max_write_bytes {
                return Err(ToolError::Denied("write exceeds max_write_bytes".into()));
            }
            let path = self.path(relative, true)?;
            let original = fs::metadata(&path)
                .ok()
                .filter(|metadata| metadata.len() <= self.policy.max_write_bytes as u64)
                .and_then(|_| fs::read_to_string(&path).ok());
            preview(&WriteProposal {
                relative,
                original: original.as_deref(),
                content: content.as_bytes(),
            })?;
            // Recheck after user code ran: the callback may have changed the path.
            self.path(relative, true)?;
            let parent = path
                .parent()
                .ok_or_else(|| ToolError::Denied("missing parent".into()))?;
            let mut temporary = NamedTempFile::new_in(parent)?;
            temporary.write_all(content.as_bytes())?;
            temporary.as_file().sync_all()?;
            self.path(relative, true)?;
            temporary
                .persist(&path)
                .map_err(|err| ToolError::Io(err.error))?;
            Ok(WriteReceipt {
                relative: relative.into(),
                bytes: content.len(),
            })
        })();
        self.record(ToolAction::Write, relative, start, &result, "wrote file");
        result
    }

    /// Return only allowlisted regular files. Symlinked directories are never entered.
    pub fn list_files(&self) -> Result<Vec<String>> {
        let start = Instant::now();
        let result = (|| {
            let mut files = Vec::new();
            let mut dirs = vec![self.workspace.clone()];
            let mut visited = 0usize;
            while let Some(dir) = dirs.pop() {
                for entry in fs::read_dir(dir)? {
                    visited += 1;
                    if visited > 10_000 {
                        return Err(ToolError::Denied(
                            "workspace listing exceeds entry limit".into(),
                        ));
                    }
                    let entry = entry?;
                    let ty = entry.file_type()?;
                    if ty.is_symlink() {
                        continue;
                    }
                    if ty.is_dir() {
                        if matches!(
                            entry.file_name().to_str(),
                            Some(".git" | "target" | "node_modules" | ".venv" | ".next")
                        ) {
                            continue;
                        }
                        dirs.push(entry.path());
                    } else if ty.is_file() {
                        let rel = entry
                            .path()
                            .strip_prefix(&self.workspace)
                            .map_err(|_| ToolError::Denied("outside workspace".into()))?
                            .to_string_lossy()
                            .to_string();
                        if self.read.is_match(&rel) {
                            files.push(rel);
                        }
                        if files.len() > 4096 {
                            return Err(ToolError::Denied("file list exceeds limit".into()));
                        }
                    }
                }
            }
            files.sort();
            Ok(files)
        })();
        self.record(ToolAction::List, "", start, &result, "listed files");
        result
    }

    /// Execute only configured argv in macOS sandbox-exec; no shell is involved.
    pub fn run_named_task(&self, name: &str) -> Result<TaskOutput> {
        let start = Instant::now();
        let result = (|| {
            let task = self
                .policy
                .tasks
                .get(name)
                .ok_or_else(|| ToolError::Denied("unknown task".into()))?;
            self.run_sandboxed(&task.argv, task.timeout_secs, false)
        })();
        self.record_task(name, start, &result);
        result
    }

    /// Return the names available to mise from this workspace and its usual task sources.
    pub fn list_mise_tasks(&self) -> Result<Vec<String>> {
        let start = Instant::now();
        let result = (|| {
            if !self.policy.mise_tasks {
                return Err(ToolError::Denied("mise tasks are not enabled".into()));
            }
            let args = vec![
                "mise".into(),
                "tasks".into(),
                "ls".into(),
                "--name-only".into(),
                "--hidden".into(),
            ];
            let output = self.run_sandboxed(&args, 15, true)?;
            if !output.success || output.truncated {
                return Err(ToolError::Denied(format!(
                    "could not list mise tasks: {}",
                    output.output.chars().take(512).collect::<String>()
                )));
            }
            Ok(output
                .output
                .lines()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect())
        })();
        self.record(
            ToolAction::List,
            "mise tasks",
            start,
            &result,
            "listed mise tasks",
        );
        result
    }

    /// Run an exact task name returned by mise, with no model-supplied command text or arguments.
    pub fn run_mise_task(&self, name: &str) -> Result<TaskOutput> {
        let start = Instant::now();
        let result = (|| {
            if name.is_empty() || name.len() > 128 || name.starts_with('-') || name.contains(":::")
            {
                return Err(ToolError::Denied("invalid mise task name".into()));
            }
            if !self.list_mise_tasks()?.iter().any(|task| task == name) {
                return Err(ToolError::Denied(format!("unknown mise task: {name}")));
            }
            self.run_sandboxed(&["mise".into(), "run".into(), name.into()], 180, true)
        })();
        self.record_task(name, start, &result);
        result
    }

    fn record_task(&self, name: &str, start: Instant, result: &Result<TaskOutput>) {
        let event_result = match result {
            Ok(output) if !output.success => {
                Err(ToolError::Denied("task exited unsuccessfully".into()))
            }
            Ok(_) => Ok(()),
            Err(error) => Err(ToolError::Denied(error.to_string())),
        };
        self.record(ToolAction::Task, name, start, &event_result, "ran task");
    }

    fn run_sandboxed(&self, argv: &[String], timeout_secs: u64, mise: bool) -> Result<TaskOutput> {
        if argv.is_empty() {
            return Err(ToolError::Policy("task argv must not be empty".into()));
        }
        if !cfg!(target_os = "macos") || !Path::new("/usr/bin/sandbox-exec").is_file() {
            return Err(ToolError::SandboxUnavailable);
        }
        let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let root = escape(&self.workspace.to_string_lossy());
        let home_dir = std::env::var("HOME").unwrap_or_default();
        let home = escape(&home_dir);
        // sandbox-exec is defense in depth, not a VM. Tasks can read most host
        // files. Mise also needs its global config, so exclude only known
        // credential directories under .config in that mode.
        let config_denials = if mise {
            ["gh", "gcloud", "aws", "azure", "1Password", "pi"]
                .iter()
                .map(|dir| format!(" (subpath \"{home}/.config/{dir}\")"))
                .collect::<String>()
        } else {
            format!(" (subpath \"{home}/.config\")")
        };
        let profile = format!(
            "(version 1)\n(allow default)\n(deny network*)\n(deny file-write*)\n(allow file-write* (subpath \"{root}\"))\n(deny file-read* (subpath \"{home}/.ssh\") (subpath \"{home}/.aws\") (subpath \"{home}/.pi\"){config_denials})\n"
        );
        // Never forward the parent's API keys or ambient secrets to tasks.
        let mut command = Command::new("/usr/bin/sandbox-exec");
        command.env_clear();
        for key in ["PATH", "CARGO_HOME", "RUSTUP_HOME", "LANG", "LC_ALL"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        let tmp = self.workspace.join("tmp");
        fs::create_dir_all(&tmp)?;
        if mise {
            let config_dir = std::env::var_os("MISE_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&home_dir).join(".config/mise"));
            let global_config = std::env::var_os("MISE_GLOBAL_CONFIG_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|| config_dir.join("config.toml"));
            command.env("MISE_CONFIG_DIR", &config_dir);
            if global_config.is_file() {
                command.env("MISE_GLOBAL_CONFIG_FILE", global_config);
            }
            let data_dir = std::env::var_os("MISE_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&home_dir).join(".local/share/mise"));
            command
                .env("MISE_DATA_DIR", data_dir)
                .env("MISE_CACHE_DIR", tmp.join("mise-cache"))
                .env("MISE_STATE_DIR", tmp.join("mise-state"))
                .env("MISE_TRUSTED_CONFIG_PATHS", &self.workspace);
        }
        command
            .arg("-p")
            .arg(profile)
            .arg(&argv[0])
            .args(&argv[1..])
            .current_dir(&self.workspace)
            .env("HOME", &self.workspace)
            .env("TMPDIR", &tmp)
            .env("GOCACHE", tmp.join("go-cache"))
            .env("GOTELEMETRY", "off")
            .env("CARGO_TARGET_DIR", self.workspace.join("target"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let out = thread::spawn(move || capture(stdout));
        let err = thread::spawn(move || capture(stderr));
        let status = child.wait_timeout(Duration::from_secs(timeout_secs))?;
        if status.is_none() {
            // Kill the entire task tree, not only sandbox-exec's wrapper.
            #[cfg(unix)]
            {
                // SAFETY: this is the process group created for this child.
                unsafe { libc::killpg(child.id() as i32, libc::SIGKILL) };
            }
            let _ = child.kill();
            let _ = child.wait();
        }
        let (mut output, out_truncated) = out
            .join()
            .map_err(|_| ToolError::Denied("output reader failed".into()))??;
        let (errors, err_truncated) = err
            .join()
            .map_err(|_| ToolError::Denied("error reader failed".into()))??;
        let remaining = MAX_OUTPUT_BYTES.saturating_sub(output.len());
        let truncated = out_truncated || err_truncated || errors.len() > remaining;
        output.extend_from_slice(&errors[..errors.len().min(remaining)]);
        if status.is_none() {
            return Err(ToolError::Timeout(timeout_secs));
        }
        let status = status.unwrap();
        Ok(TaskOutput {
            success: status.success(),
            exit_code: status.code(),
            output: String::from_utf8_lossy(&output).into_owned(),
            truncated,
        })
    }
}

fn capture(mut stream: impl Read) -> io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::new();
    let mut truncated = false;
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        let take = n.min(MAX_OUTPUT_BYTES.saturating_sub(output.len()));
        output.extend_from_slice(&chunk[..take]);
        truncated |= take < n;
    }
    Ok((output, truncated))
}

fn is_shell(binary: &str) -> bool {
    matches!(
        Path::new(binary).file_name().and_then(|s| s.to_str()),
        Some("sh" | "bash" | "zsh" | "fish" | "dash" | "ksh" | "csh" | "tcsh" | "env")
    )
}

fn compile_globs(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if pattern.is_empty()
            || Path::new(pattern).components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ToolError::Policy(format!(
                "invalid allowlist glob: {pattern}"
            )));
        }
        builder.add(Glob::new(pattern).map_err(|e| ToolError::Policy(e.to_string()))?);
    }
    builder
        .build()
        .map_err(|e| ToolError::Policy(e.to_string()))
}
