use crate::{App, UiEvent};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
};
use diffagent_core::events::{Event as AgentEvent, QuestionAnswer, QuestionService};
use std::{
    io::{self, Write},
    process::{Command, Stdio},
    sync::mpsc::{Receiver, Sender, TryRecvError},
    time::Duration,
};

struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableBracketedPaste, DisableMouseCapture);
        ratatui::restore();
    }
}

/// Runs on the calling terminal thread. Producers may send events from any thread;
/// polling keyboard with a short timeout keeps rendering responsive between deltas.
/// The caller owns both channel endpoints and any backend work.
pub fn run(events: Receiver<AgentEvent>, submissions: Sender<String>) -> io::Result<()> {
    run_inner(events, submissions, None)
}

pub fn run_with_questions(
    events: Receiver<AgentEvent>,
    submissions: Sender<String>,
    service: QuestionService,
) -> io::Result<()> {
    let result = run_inner(events, submissions, Some(service.clone()));
    service.cancel_all();
    result
}

fn run_inner(
    events: Receiver<AgentEvent>,
    submissions: Sender<String>,
    service: Option<QuestionService>,
) -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let _restore = Restore;
    execute!(io::stdout(), EnableBracketedPaste, EnableMouseCapture)?;
    let mut app = service.map_or_else(App::new, App::with_questions);
    let mut events_open = true;
    loop {
        while events_open {
            match events.try_recv() {
                Ok(event) => app.apply(UiEvent::from(event)),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    app.apply(UiEvent::Status("Agent event channel closed".into()));
                    events_open = false;
                    break;
                }
            }
        }
        terminal.draw(|frame| app.render(frame))?;
        if app.should_quit() {
            return Ok(());
        }
        if event::poll(Duration::from_millis(30))? {
            match event::read()? {
                Event::Key(key)
                    if key.kind == KeyEventKind::Press
                        && key.code == KeyCode::Char('y')
                        && key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    let message = match app.copy_text() {
                        Some((text, label)) => match copy_to_clipboard(&text) {
                            Ok(()) => format!("Copied {label}"),
                            Err(error) => format!("Copy failed: {error}"),
                        },
                        None => "Nothing to copy".into(),
                    };
                    app.copy_feedback(message);
                }
                Event::Key(key) if !app.handle_key(key, &submissions) => return Ok(()),
                Event::Paste(text) => app.paste(&text),
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                _ => {}
            }
        }
    }
}

/// Send plain text to the system clipboard without a shell or terminal escape sequence.
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let commands: &[(&str, &[&str])] = &[("pbcopy", &[])];
    #[cfg(target_os = "linux")]
    let commands: &[(&str, &[&str])] = &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"])];
    #[cfg(target_os = "windows")]
    let commands: &[(&str, &[&str])] = &[("clip.exe", &[])];
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let commands: &[(&str, &[&str])] = &[];

    let mut last_error = io::Error::new(io::ErrorKind::NotFound, "no clipboard command available");
    for (command, args) in commands {
        let mut child = match Command::new(command)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                last_error = error;
                continue;
            }
        };
        let written = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("clipboard command has no input pipe"))?
            .write_all(text.as_bytes());
        let status = child.wait()?;
        if written.is_ok() && status.success() {
            return Ok(());
        }
        last_error = written
            .err()
            .unwrap_or_else(|| io::Error::other(format!("{command} exited with status {status}")));
    }
    Err(last_error)
}

/// Interactive YAML flows ask on stderr without corrupting JSON events on stdout.
/// Headless flows return a structured skip without waiting for stdin.
pub fn ask_terminal(question: &str, options: &[String]) -> QuestionAnswer {
    use std::io::IsTerminal;
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return QuestionAnswer::skipped("non-interactive flow");
    }
    if question.trim().is_empty()
        || question.len() > 4096
        || options.len() > 4
        || options
            .iter()
            .any(|option| option.trim().is_empty() || option.len() > 256)
    {
        return QuestionAnswer::skipped("invalid question or options");
    }
    eprintln!("\nAgent asks: {}", crate::sanitize::visible(question));
    for (index, option) in options.iter().enumerate() {
        eprintln!("  {}. {}", index + 1, crate::sanitize::visible(option));
    }
    eprint!("Answer (number, custom text, or /skip): ");
    let _ = io::stderr().flush();
    let mut answer = String::new();
    match io::stdin().read_line(&mut answer) {
        Ok(0) | Err(_) => QuestionAnswer::skipped("no user input"),
        Ok(_) => {
            let answer = answer.trim();
            if answer.is_empty() || answer == "/skip" {
                return QuestionAnswer::skipped("user skipped the question");
            }
            if let Ok(index) = answer.parse::<usize>()
                && (1..=options.len()).contains(&index)
            {
                return QuestionAnswer::Answered {
                    answer: options[index - 1].clone(),
                };
            }
            if answer.len() > 4096 {
                return QuestionAnswer::skipped("answer exceeds 4096 bytes");
            }
            QuestionAnswer::Answered {
                answer: answer.into(),
            }
        }
    }
}
