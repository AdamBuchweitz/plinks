use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use anyhow::Result;
use clap::Parser;
use plinks::cli::Cli;
use plinks::config::Config;
use plinks::open_link::LinkOpener;
use tempfile::TempDir;

struct RecordingOpener {
    urls: RefCell<Vec<String>>,
}

impl RecordingOpener {
    fn new() -> Self {
        Self {
            urls: RefCell::new(Vec::new()),
        }
    }

    fn take(&self) -> Vec<String> {
        self.urls.borrow().clone()
    }
}

impl LinkOpener for RecordingOpener {
    fn open(&self, url: &str) -> Result<()> {
        self.urls.borrow_mut().push(url.to_string());
        Ok(())
    }
}

fn run(args: &[&str], cwd: &Path, opener: &RecordingOpener) -> Result<String> {
    let cli = Cli::parse_from(std::iter::once("plinks").chain(args.iter().copied()));
    let mut stdout = Vec::new();
    plinks::run(cli, cwd, opener, &mut stdout)?;
    Ok(String::from_utf8(stdout).unwrap())
}

fn load_config(path: &Path) -> Config {
    let raw = fs::read_to_string(path).unwrap();
    toml::from_str::<Config>(&raw)
        .unwrap()
        .validate_and_normalize()
        .unwrap()
}

fn help_output(args: &[&str]) -> String {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_plinks"))
        .args(args)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "help command failed with status {}.\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn add_creates_file_when_absent() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(&["add", "docs", "https://docs.rs"], temp.path(), &opener).unwrap();

    assert!(temp.path().join("project-links.toml").exists());
}

#[test]
fn add_stores_aliases_tags_and_note() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &[
            "add",
            "docs",
            "https://docs.rs",
            "--alias",
            "api",
            "--alias",
            "rust",
            "--tag",
            "docs",
            "--tag",
            "rust",
            "--note",
            "Reference",
        ],
        temp.path(),
        &opener,
    )
    .unwrap();

    let config = load_config(&temp.path().join("project-links.toml"));
    let entry = &config.links["docs"];
    assert_eq!(entry.aliases, vec!["api", "rust"]);
    assert_eq!(entry.tags, vec!["docs", "rust"]);
    assert_eq!(entry.note.as_deref(), Some("Reference"));
}

#[test]
fn add_rejects_collisions() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &["add", "docs", "https://docs.rs", "--alias", "api"],
        temp.path(),
        &opener,
    )
    .unwrap();
    let err = run(
        &["add", "db", "https://db.example", "--alias", "api"],
        temp.path(),
        &opener,
    )
    .unwrap_err();

    assert!(err.to_string().contains("alias 'api'"));
}

#[test]
fn add_force_replaces_same_primary_entry() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &["add", "docs", "https://docs.rs", "--alias", "api"],
        temp.path(),
        &opener,
    )
    .unwrap();
    run(
        &[
            "add",
            "docs",
            "https://example.com/new",
            "--alias",
            "guide",
            "--force",
        ],
        temp.path(),
        &opener,
    )
    .unwrap();

    let config = load_config(&temp.path().join("project-links.toml"));
    let entry = &config.links["docs"];
    assert_eq!(entry.url, "https://example.com/new");
    assert_eq!(entry.aliases, vec!["guide"]);
}

#[test]
fn open_resolves_primary_and_alias() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &["add", "docs", "https://docs.rs", "--alias", "api"],
        temp.path(),
        &opener,
    )
    .unwrap();
    run(&["open", "docs"], temp.path(), &opener).unwrap();
    run(&["open", "api"], temp.path(), &opener).unwrap();

    assert_eq!(
        opener.take(),
        vec!["https://docs.rs".to_string(), "https://docs.rs".to_string()]
    );
}

#[test]
fn open_tag_selects_all_matches_in_sorted_order() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &["add", "b", "https://b.example", "--tag", "shared"],
        temp.path(),
        &opener,
    )
    .unwrap();
    run(
        &["add", "a", "https://a.example", "--tag", "shared"],
        temp.path(),
        &opener,
    )
    .unwrap();

    run(&["open", "--tag", "shared"], temp.path(), &opener).unwrap();
    assert_eq!(
        opener.take(),
        vec![
            "https://a.example".to_string(),
            "https://b.example".to_string()
        ]
    );
}

#[test]
fn list_filters_and_displays_rows() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &[
            "add",
            "docs",
            "https://docs.rs",
            "--alias",
            "api",
            "--tag",
            "rust",
        ],
        temp.path(),
        &opener,
    )
    .unwrap();
    run(
        &["add", "jira", "https://jira.example", "--tag", "ops"],
        temp.path(),
        &opener,
    )
    .unwrap();

    let output = run(&["list", "--tag", "rust"], temp.path(), &opener).unwrap();
    assert!(output.contains("PRIMARY"));
    assert!(output.contains("docs"));
    assert!(output.contains("api"));
    assert!(!output.contains("jira"));
}

#[test]
fn ls_alias_runs_list_command() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &[
            "add",
            "docs",
            "https://docs.rs",
            "--alias",
            "api",
            "--tag",
            "rust",
        ],
        temp.path(),
        &opener,
    )
    .unwrap();

    let output = run(&["ls", "--tag", "rust"], temp.path(), &opener).unwrap();
    assert!(output.contains("PRIMARY"));
    assert!(output.contains("docs"));
    assert!(output.contains("api"));
}

#[test]
fn remove_deletes_by_primary_only() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &["add", "docs", "https://docs.rs", "--alias", "api"],
        temp.path(),
        &opener,
    )
    .unwrap();
    run(&["remove", "docs"], temp.path(), &opener).unwrap();

    let config = load_config(&temp.path().join("project-links.toml"));
    assert!(config.links.is_empty());
}

#[test]
fn remove_does_not_accept_extra_alias() {
    let temp = TempDir::new().unwrap();
    let opener = RecordingOpener::new();

    run(
        &["add", "docs", "https://docs.rs", "--alias", "api"],
        temp.path(),
        &opener,
    )
    .unwrap();
    let err = run(&["remove", "api"], temp.path(), &opener).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn git_root_resolution_is_used_from_nested_directory() {
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let nested = repo.join("nested/work");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(&nested).unwrap();

    let opener = RecordingOpener::new();
    run(&["add", "docs", "https://docs.rs"], &nested, &opener).unwrap();

    assert!(repo.join("project-links.toml").exists());
}

#[test]
fn top_level_help_lists_command_summaries_and_examples() {
    let help = help_output(&["-h"]);

    assert!(help.contains("Manage shared project links from the command line or TUI"));
    assert!(help.contains("open"));
    assert!(help.contains("Open a saved link by primary name, alias, or tag"));
    assert!(help.contains("list"));
    assert!(help.contains("List saved links"));
    assert!(help.contains("add"));
    assert!(help.contains("Add a saved link"));
    assert!(help.contains("remove"));
    assert!(help.contains("Remove a saved link by primary name"));
    assert!(help.contains("manage"));
    assert!(help.contains("Open the interactive terminal UI"));
    assert!(help.contains("plinks help <command>"));
}

#[test]
fn no_args_defaults_to_manage_command() {
    let cli = Cli::parse_from(["plinks"]);
    assert!(cli.command.is_none());
}

#[test]
fn add_help_explains_primary_name_and_options() {
    let help = help_output(&["add", "-h"]);

    assert!(
        help.contains("Add a saved link"),
        "unexpected add help output:\n{help}"
    );
    assert!(
        help.contains("PRIMARY_NAME"),
        "unexpected add help output:\n{help}"
    );
    assert!(help.contains("URL"), "unexpected add help output:\n{help}");
    assert!(
        help.contains("Stable primary name for the link"),
        "unexpected add help output:\n{help}"
    );
    assert!(
        help.contains("Additional name that can also be used with `plinks open`"),
        "unexpected add help output:\n{help}"
    );
    assert!(
        help.contains("Tag used to group links"),
        "unexpected add help output:\n{help}"
    );
    assert!(
        help.contains("plinks add docs https://docs.rs --alias api --tag rust"),
        "unexpected add help output:\n{help}"
    );
}

#[test]
fn open_help_explains_name_alias_and_tag_modes() {
    let help = help_output(&["open", "-h"]);

    assert!(
        help.contains("Open a saved link by primary name, alias, or tag"),
        "unexpected open help output:\n{help}"
    );
    assert!(
        help.contains("NAME_OR_ALIAS"),
        "unexpected open help output:\n{help}"
    );
    assert!(
        help.contains("--tag"),
        "unexpected open help output:\n{help}"
    );
    assert!(
        help.contains("Primary name or alias of the saved link to open"),
        "unexpected open help output:\n{help}"
    );
    assert!(
        help.contains("Open every saved link with this tag"),
        "unexpected open help output:\n{help}"
    );
    assert!(
        help.contains("plinks open --tag rust"),
        "unexpected open help output:\n{help}"
    );
}
