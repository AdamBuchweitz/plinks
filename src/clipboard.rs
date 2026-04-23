use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};

pub trait LinkClipboard {
    fn copy_text(&self, text: &str) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemClipboard;

impl LinkClipboard for SystemClipboard {
    fn copy_text(&self, text: &str) -> Result<()> {
        let candidates = clipboard_command_candidates();
        let mut errors = Vec::new();

        for candidate in candidates {
            match run_clipboard_command(&candidate, text) {
                Ok(()) => return Ok(()),
                Err(err) => errors.push(format!("{}: {err}", candidate.program)),
            }
        }

        bail!(
            "failed to copy to the system clipboard; tried {}",
            errors.join("; ")
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClipboardCommand {
    program: &'static str,
    args: &'static [&'static str],
}

const COMMAND_SETTLE_TIMEOUT: Duration = Duration::from_millis(200);

fn run_clipboard_command(command: &ClipboardCommand, text: &str) -> Result<()> {
    let mut child = Command::new(command.program)
        .args(command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {:?}", command_line(command)))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(text.as_bytes())
            .with_context(|| format!("failed to write to {:?}", command_line(command)))?;
    } else {
        return Err(anyhow!(
            "clipboard command {:?} has no stdin",
            command.program
        ));
    }

    // Clipboard tools that read from stdin will block until they receive EOF.
    // Drop the pipe before waiting so they can finish or daemonize.
    drop(child.stdin.take());

    wait_for_clipboard_command(child, command, COMMAND_SETTLE_TIMEOUT)
}

fn wait_for_clipboard_command(
    mut child: Child,
    command: &ClipboardCommand,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("failed to check {:?}", command_line(command)))?
        {
            return handle_clipboard_command_exit(child, command, status);
        }

        if Instant::now() >= deadline {
            // Long-lived clipboard tools usually keep running so the copied value
            // remains available after this process exits.
            thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(());
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn handle_clipboard_command_exit(
    mut child: Child,
    command: &ClipboardCommand,
    status: ExitStatus,
) -> Result<()> {
    if status.success() {
        return Ok(());
    }

    let mut stderr = String::new();
    if let Some(mut child_stderr) = child.stderr.take() {
        child_stderr
            .read_to_string(&mut stderr)
            .with_context(|| format!("failed to read stderr from {:?}", command_line(command)))?;
    }
    let stderr = stderr.trim().to_string();
    if stderr.is_empty() {
        bail!("clipboard command exited with {}", status);
    }
    bail!("{stderr}");
}

#[cfg(target_os = "macos")]
fn clipboard_command_candidates() -> Vec<ClipboardCommand> {
    vec![ClipboardCommand {
        program: "pbcopy",
        args: &[],
    }]
}

#[cfg(target_os = "windows")]
fn clipboard_command_candidates() -> Vec<ClipboardCommand> {
    vec![ClipboardCommand {
        program: "clip",
        args: &[],
    }]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn clipboard_command_candidates() -> Vec<ClipboardCommand> {
    let has_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let has_x11 = std::env::var_os("DISPLAY").is_some();

    linux_candidates_from_flags(has_wayland, has_x11)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_candidates_from_flags(wayland: bool, x11: bool) -> Vec<ClipboardCommand> {
    let mut candidates = Vec::new();

    if wayland {
        candidates.push(ClipboardCommand {
            program: "wl-copy",
            args: &[],
        });
    }

    if x11 {
        candidates.push(ClipboardCommand {
            program: "xclip",
            args: &["-selection", "clipboard", "-in"],
        });
        candidates.push(ClipboardCommand {
            program: "xsel",
            args: &["--clipboard", "--input"],
        });
    }

    if !wayland && !x11 {
        candidates.push(ClipboardCommand {
            program: "wl-copy",
            args: &[],
        });
        candidates.push(ClipboardCommand {
            program: "xclip",
            args: &["-selection", "clipboard", "-in"],
        });
        candidates.push(ClipboardCommand {
            program: "xsel",
            args: &["--clipboard", "--input"],
        });
    }

    candidates
}

fn command_line(command: &ClipboardCommand) -> String {
    if command.args.is_empty() {
        command.program.to_string()
    } else {
        format!("{} {}", command.program, command.args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn prefers_wayland_clipboard_when_available() {
        let candidates = linux_candidates_from_flags(true, false);
        assert_eq!(
            candidates,
            vec![ClipboardCommand {
                program: "wl-copy",
                args: &[],
            }]
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn prefers_x11_clipboards_when_display_is_set() {
        let candidates = linux_candidates_from_flags(false, true);
        assert_eq!(
            candidates,
            vec![
                ClipboardCommand {
                    program: "xclip",
                    args: &["-selection", "clipboard", "-in"],
                },
                ClipboardCommand {
                    program: "xsel",
                    args: &["--clipboard", "--input"],
                }
            ]
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn falls_back_to_common_linux_clipboard_tools_without_session_hints() {
        let candidates = linux_candidates_from_flags(false, false);
        assert_eq!(
            candidates,
            vec![
                ClipboardCommand {
                    program: "wl-copy",
                    args: &[],
                },
                ClipboardCommand {
                    program: "xclip",
                    args: &["-selection", "clipboard", "-in"],
                },
                ClipboardCommand {
                    program: "xsel",
                    args: &["--clipboard", "--input"],
                }
            ]
        );
    }
}
