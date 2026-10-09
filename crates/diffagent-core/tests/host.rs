use diffagent_core::spec::{LoadedAgent, TaskSpec, ToolSpec};
use diffagent_core::tools::{ToolAction, ToolError, ToolHost};
use std::{collections::BTreeMap, fs};

fn host(dir: &tempfile::TempDir) -> ToolHost {
    ToolHost::new(
        dir.path(),
        ToolSpec {
            read: vec!["src/**".into()],
            write: vec!["src/**".into()],
            mise_tasks: false,
            tasks: BTreeMap::new(),
            max_write_bytes: 100,
        },
    )
    .unwrap()
}

#[test]
fn preview_sees_full_content_before_atomic_commit() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/a.txt"), "old").unwrap();
    let h = host(&dir);
    let receipt = h
        .write_file("src/a.txt", "new content", |proposal| {
            assert_eq!(proposal.relative, "src/a.txt");
            assert_eq!(proposal.original, Some("old"));
            assert_eq!(proposal.content, b"new content");
            assert_eq!(fs::read(dir.path().join("src/a.txt")).unwrap(), b"old");
            Ok(())
        })
        .unwrap();
    assert_eq!(receipt.bytes, 11);
    assert_eq!(h.read_file("src/a.txt").unwrap(), "new content");
    assert_eq!(h.list_files().unwrap(), vec!["src/a.txt"]);
    assert!(
        h.events()
            .iter()
            .any(|e| e.action == ToolAction::Write && e.success)
    );
}

#[test]
fn failed_preview_and_invalid_write_preserve_old_content() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    let path = dir.path().join("src/a.txt");
    fs::write(&path, "old").unwrap();
    let h = host(&dir);
    assert!(
        h.write_file("src/a.txt", "replacement", |_| Err(ToolError::Preview(
            "cancelled".into()
        )))
        .is_err()
    );
    assert!(
        h.write_file("src/a.txt", &"x".repeat(101), |_| panic!(
            "no preview for denied write"
        ))
        .is_err()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "old");
}

#[test]
fn failed_commit_does_not_replace_other_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    let h = host(&dir);
    h.write_file("src/a", "first", |_| Ok(())).unwrap();
    fs::create_dir(dir.path().join("src/occupied")).unwrap();
    assert!(h.write_file("src/occupied", "second", |_| Ok(())).is_err());
    assert_eq!(
        fs::read_to_string(dir.path().join("src/a")).unwrap(),
        "first"
    );
    assert_eq!(h.list_files().unwrap(), vec!["src/a"]);
}

#[test]
fn benchmark_policy_hides_external_acceptance_file() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let agent = LoadedAgent::load(&root.join("agent-benchmark.yaml")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("tests")).unwrap();
    fs::write(dir.path().join("tests/acceptance.rs"), "hidden").unwrap();
    let host = ToolHost::new(dir.path(), agent.spec.tools).unwrap();
    assert!(matches!(
        host.read_file("tests/acceptance.rs"),
        Err(ToolError::Denied(_))
    ));
    assert!(matches!(
        host.write_file("tests/acceptance.rs", "overwrite", |_| Ok(())),
        Err(ToolError::Denied(_))
    ));
    assert_eq!(
        fs::read_to_string(dir.path().join("tests/acceptance.rs")).unwrap(),
        "hidden"
    );
    assert!(host.list_mise_tasks().is_err());
    assert!(host.run_mise_task("check").is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn authoring_a_mise_task_makes_it_available_and_runnable() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = ToolSpec::default();
    policy.mise_tasks = true;
    policy.write = vec!["**".into()];
    let host = ToolHost::new(dir.path(), policy).unwrap();
    host.write_file(
        "mise.toml",
        "[tasks.smoke]\nrun = \"printf 'mise task ran\\n'\"\n\n[tasks.hidden]\nhide = true\nrun = \"printf 'hidden task ran\\n'\"\n",
        |_| Ok(()),
    )
    .unwrap();
    let names = host.list_mise_tasks().unwrap();
    assert!(names.contains(&"smoke".into()));
    assert!(names.contains(&"hidden".into()));
    let output = host.run_mise_task("smoke").unwrap();
    assert!(output.success, "{}", output.output);
    assert!(output.output.contains("mise task ran"), "{}", output.output);
    assert!(host.run_mise_task("not-a-task").is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn available_global_mise_tasks_are_listed() {
    let global = std::process::Command::new("mise")
        .args(["tasks", "ls", "--name-only", "--global"])
        .output()
        .unwrap();
    if !global.status.success() {
        return;
    }
    let names = String::from_utf8_lossy(&global.stdout);
    let Some(global_task) = names.lines().next() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut policy = ToolSpec::default();
    policy.mise_tasks = true;
    let host = ToolHost::new(dir.path(), policy).unwrap();
    assert!(
        host.list_mise_tasks()
            .unwrap()
            .iter()
            .any(|task| task == global_task)
    );
}

#[test]
fn chat_policy_allows_go_files_across_the_workspace() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let agent = LoadedAgent::load(&root.join("agent.yaml")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("cmd")).unwrap();
    fs::write(dir.path().join("go.mod"), "module example.com/test\n").unwrap();
    let host = ToolHost::new(dir.path(), agent.spec.tools).unwrap();
    assert_eq!(
        host.read_file("go.mod").unwrap(),
        "module example.com/test\n"
    );
    host.write_file("main.go", "package main\n", |_| Ok(()))
        .unwrap();
    host.write_file("cmd/server.go", "package main\n", |_| Ok(()))
        .unwrap();
    host.write_file(".hidden.go", "package main\n", |_| Ok(()))
        .unwrap();
    assert_eq!(host.read_file("cmd/server.go").unwrap(), "package main\n");
    assert_eq!(host.read_file(".hidden.go").unwrap(), "package main\n");
    assert!(host.list_files().unwrap().contains(&"main.go".into()));
    assert!(host.read_file("../outside").is_err());
    assert!(host.write_file("../outside", "no", |_| Ok(())).is_err());
}

#[test]
fn paths_and_allowlist_are_enforced() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    let h = host(&dir);
    for path in ["../outside", "src/../outside", "/tmp/outside", "src/./a"] {
        assert!(h.write_file(path, "x", |_| Ok(())).is_err(), "{path}");
    }
    assert!(h.read_file("Cargo.toml").is_err());
    assert!(h.write_file("other.txt", "x", |_| Ok(())).is_err());
    assert!(h.run_named_task("unconfigured").is_err());
}

#[cfg(unix)]
#[test]
fn symlink_file_and_directory_escape_are_denied() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(outside.path().join("secret"), "private").unwrap();
    symlink(outside.path(), dir.path().join("src/linked_dir")).unwrap();
    symlink(
        outside.path().join("secret"),
        dir.path().join("src/linked_file"),
    )
    .unwrap();
    let h = host(&dir);
    for path in ["src/linked_dir/secret", "src/linked_file"] {
        assert!(h.read_file(path).is_err());
        assert!(h.write_file(path, "leak", |_| Ok(())).is_err());
    }
    assert!(h.list_files().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(outside.path().join("secret")).unwrap(),
        "private"
    );
}

#[test]
fn callback_path_change_aborts_before_commit() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    let h = host(&dir);
    let path = dir.path().join("src/a");
    fs::write(&path, "old").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = tempfile::tempdir().unwrap();
        let result = h.write_file("src/a", "new", |_| {
            fs::remove_file(&path).unwrap();
            symlink(outside.path().join("secret"), &path).unwrap();
            Ok(())
        });
        assert!(result.is_err());
        assert!(!outside.path().join("secret").exists());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn cargo_test_runs_inside_the_configured_workspace() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"tool-smoke\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("src/lib.rs"),
        "#[test] fn passes() { assert_eq!(2 + 2, 4); }\n",
    )
    .unwrap();
    let mut policy = ToolSpec::default();
    policy.max_write_bytes = 256 * 1024;
    policy.tasks.insert(
        "test".into(),
        TaskSpec {
            argv: vec![
                "cargo".into(),
                "test".into(),
                "--offline".into(),
                "--quiet".into(),
            ],
            timeout_secs: 45,
        },
    );
    let result = ToolHost::new(dir.path(), policy)
        .unwrap()
        .run_named_task("test")
        .unwrap();
    assert!(result.success, "{}", result.output);
}

#[cfg(target_os = "macos")]
#[test]
fn timed_out_task_terminates_its_process_group() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = ToolSpec::default();
    policy.tasks.insert(
        "wait".into(),
        TaskSpec {
            argv: vec!["/bin/sleep".into(), "5".into()],
            timeout_secs: 1,
        },
    );
    let host = ToolHost::new(dir.path(), policy).unwrap();
    let started = std::time::Instant::now();
    assert!(matches!(
        host.run_named_task("wait"),
        Err(ToolError::Timeout(1))
    ));
    assert!(
        started.elapsed().as_secs() < 4,
        "task descendants survived timeout"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn task_does_not_inherit_the_model_api_key() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = ToolSpec::default();
    policy.tasks.insert(
        "inspect".into(),
        TaskSpec {
            argv: vec!["/usr/bin/printenv".into(), "DEEPSEEK_API_KEY".into()],
            timeout_secs: 2,
        },
    );
    let result = ToolHost::new(dir.path(), policy)
        .unwrap()
        .run_named_task("inspect")
        .unwrap();
    assert!(!result.success);
    assert!(
        result.output.is_empty(),
        "API key was forwarded into the task process"
    );
}

#[test]
fn task_policy_rejects_shell_and_unsupported_platform_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let mut policy = ToolSpec::default();
    policy.max_write_bytes = 256 * 1024;
    policy.tasks.insert(
        "bad".into(),
        TaskSpec {
            argv: vec!["/bin/sh".into(), "-c".into(), "echo unsafe".into()],
            timeout_secs: 2,
        },
    );
    assert!(ToolHost::new(dir.path(), policy.clone()).is_err());
    policy.tasks.clear();
    policy.tasks.insert(
        "safe".into(),
        TaskSpec {
            argv: vec!["/usr/bin/true".into()],
            timeout_secs: 2,
        },
    );
    let h = ToolHost::new(dir.path(), policy).unwrap();
    #[cfg(not(target_os = "macos"))]
    assert!(matches!(
        h.run_named_task("safe"),
        Err(ToolError::SandboxUnavailable)
    ));
    #[cfg(target_os = "macos")]
    if std::path::Path::new("/usr/bin/sandbox-exec").is_file() {
        assert!(h.run_named_task("safe").unwrap().success);
    } else {
        assert!(matches!(
            h.run_named_task("safe"),
            Err(ToolError::SandboxUnavailable)
        ));
    }
}
