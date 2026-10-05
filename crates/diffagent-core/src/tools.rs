use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::Serialize;
use wait_timeout::ChildExt;

#[derive(Clone)]
pub struct ToolHost {
    workspace: PathBuf,
    events: Arc<Mutex<Vec<ToolEvent>>>,
}

#[derive(Clone, Serialize)]
pub struct ToolEvent {
    pub name: String,
    pub argument: String,
    pub result: String,
    pub elapsed_ms: u128,
    pub success: bool,
}

impl ToolHost {
    pub fn new(workspace: PathBuf, acceptance: Option<&Path>) -> io::Result<Self> {
        fs::create_dir_all(workspace.join("src"))?;
        fs::write(
            workspace.join("Cargo.toml"),
            "[package]\nname = \"generated-example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        )?;
        fs::write(
            workspace.join("src/lib.rs"),
            "// Replace this file with the requested implementation and tests.\n",
        )?;
        if let Some(source) = acceptance {
            fs::create_dir_all(workspace.join("tests"))?;
            fs::copy(source, workspace.join("tests/acceptance.rs"))?;
        }
        Ok(Self {
            workspace,
            events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn events(&self) -> Vec<ToolEvent> {
        self.events.lock().unwrap().clone()
    }
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    fn path(&self, requested: &str, writable: bool) -> io::Result<PathBuf> {
        let relative = Path::new(requested);
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || relative.as_os_str().is_empty()
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "only relative paths without traversal are allowed",
            ));
        }
        if writable && requested != "src/lib.rs" {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "only src/lib.rs can be written",
            ));
        }
        if !matches!(requested, "src/lib.rs" | "Cargo.toml") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "only src/lib.rs and Cargo.toml can be read",
            ));
        }
        // A generated test may modify the workspace. Do not let a subsequent
        // host-side tool follow a symlink out of it.
        if requested.starts_with("src/")
            && fs::symlink_metadata(self.workspace.join("src"))?
                .file_type()
                .is_symlink()
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "src is a symlink",
            ));
        }
        let path = self.workspace.join(relative);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "file is a symlink",
            ));
        }
        Ok(path)
    }

    fn record(&self, name: &str, argument: &str, start: Instant, result: &io::Result<String>) {
        self.events.lock().unwrap().push(ToolEvent {
            name: name.into(),
            argument: argument.into(),
            elapsed_ms: start.elapsed().as_millis(),
            success: result
                .as_ref()
                .is_ok_and(|s| name != "run_task" || s.starts_with("PASS:")),
            result: match result {
                Ok(s) => s.chars().take(4000).collect(),
                Err(e) => e.to_string(),
            },
        });
    }

    pub fn read_file(&self, path: &str) -> io::Result<String> {
        let start = Instant::now();
        let result = (|| {
            let content = fs::read_to_string(self.path(path, false)?)?;
            if content.len() > 32_000 {
                return Err(io::Error::other("file exceeds 32 KB"));
            }
            Ok(content)
        })();
        self.record("read_file", path, start, &result);
        result
    }

    pub fn write_file(&self, path: &str, content: &str) -> io::Result<String> {
        let start = Instant::now();
        let result = (|| {
            if content.len() > 32_000 {
                return Err(io::Error::other("file exceeds 32 KB"));
            }
            fs::write(self.path(path, true)?, content)?;
            Ok(format!("wrote {path} ({} bytes)", content.len()))
        })();
        self.record("write_file", path, start, &result);
        result
    }

    /// Fixed allowlisted task, never a model-supplied shell command.
    /// This proof of concept requires macOS sandbox-exec; it refuses to execute otherwise.
    pub fn run_task(&self, task: &str) -> io::Result<String> {
        let start = Instant::now();
        let result = (|| {
            if task != "test" {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "only task=test is allowed",
                ));
            }
            if !cfg!(target_os = "macos") || !Path::new("/usr/bin/sandbox-exec").exists() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "sandbox-exec unavailable: refusing to execute generated code",
                ));
            }
            // Restrict filesystem writes to the generated workspace and deny network access.
            // This OS profile is defense-in-depth, not a container/VM security boundary.
            let root = self.workspace.canonicalize()?;
            let home = std::env::var("HOME").unwrap_or_default();
            let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
            // macOS toolchains need system services to compile. Deny network and
            // writes outside the workspace; hide common credential directories.
            let profile = format!(
                "(version 1)\n(allow default)\n(deny network*)\n(deny file-write*)\n(allow file-write* (subpath \"{}\"))\n(deny file-read* (subpath \"{}/.ssh\") (subpath \"{}/.aws\") (subpath \"{}/.config\") (subpath \"{}/.pi\"))\n",
                escape(&root.display().to_string()),
                escape(&home),
                escape(&home),
                escape(&home),
                escape(&home)
            );
            let profile_path = root.join("sandbox.sb");
            fs::write(&profile_path, profile)?;
            let output_path = root.join("task-output.txt");
            let output = fs::File::create(&output_path)?;
            let errors = output.try_clone()?;
            fs::create_dir_all(root.join("tmp"))?;
            let mut child = Command::new("/usr/bin/sandbox-exec")
                .arg("-f")
                .arg(&profile_path)
                .arg("cargo")
                .arg("test")
                .arg("--offline")
                .arg("--quiet")
                .current_dir(&root)
                .env("CARGO_TARGET_DIR", root.join("target"))
                .env("TMPDIR", root.join("tmp"))
                .env("HOME", &root)
                .stdout(Stdio::from(output))
                .stderr(Stdio::from(errors))
                .spawn()?;
            let status = match child.wait_timeout(Duration::from_secs(45))? {
                Some(status) => status,
                None => {
                    child.kill()?;
                    child.wait()?;
                    return Err(io::Error::other("cargo test timed out after 45s"));
                }
            };
            let text = fs::read_to_string(&output_path)?
                .chars()
                .take(6000)
                .collect::<String>();
            let text = text.trim();
            if status.success() {
                Ok(format!("PASS:\n{text}\n"))
            } else {
                Ok(format!("FAIL:\n{text}\n"))
            }
        })();
        self.record("run_task", task, start, &result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn executes_allowlisted_tests_in_sandbox() {
        let dir =
            std::env::temp_dir().join(format!("diffagent-sandbox-test-{}", std::process::id()));
        let host = ToolHost::new(dir.clone(), None).unwrap();
        host.write_file(
            "src/lib.rs",
            "#[test] fn works() { assert_eq!(2 + 2, 4); }\n",
        )
        .unwrap();
        let output = host.run_task("test").unwrap();
        assert!(output.starts_with("PASS:"), "{output}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn seeds_acceptance_tests_outside_the_model_tool_surface() {
        let dir =
            std::env::temp_dir().join(format!("diffagent-acceptance-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let fixture = dir.join("fixture.rs");
        fs::write(&fixture, "#[test] fn external() { assert!(true); }\n").unwrap();
        let host = ToolHost::new(dir.join("workspace"), Some(&fixture)).unwrap();
        assert!(host.workspace().join("tests/acceptance.rs").is_file());
        assert!(host.read_file("tests/acceptance.rs").is_err());
        assert!(host.write_file("tests/acceptance.rs", "").is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_traversal_and_arbitrary_tasks() {
        let dir = std::env::temp_dir().join(format!("diffagent-tool-test-{}", std::process::id()));
        let host = ToolHost::new(dir.clone(), None).unwrap();
        assert!(host.write_file("../outside", "x").is_err());
        assert!(host.read_file("../../.ssh/id_rsa").is_err());
        assert!(host.run_task("rm -rf /").is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
