use clap::{ArgGroup, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "plinks",
    about = "Manage shared project links from the command line or TUI",
    long_about = "Manage shared project links stored in project-links.toml.\n\nUse plinks to save docs, dashboards, tickets, and runbooks under stable project-local names so the whole repository can share them.",
    after_help = "Examples:\n  plinks add docs https://docs.rs --alias api --tag rust\n  plinks open docs\n  plinks open api\n  plinks open --tag rust\n  plinks list --tag rust\n  plinks manage\n\nUse `plinks help <command>` for command-specific examples."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Open a saved link by primary name, alias, or tag")]
    Open(OpenArgs),
    #[command(alias = "ls")]
    #[command(about = "List saved links")]
    List(ListArgs),
    #[command(about = "Add a saved link")]
    Add(AddArgs),
    #[command(about = "Remove a saved link by primary name")]
    Remove(RemoveArgs),
    #[command(about = "Open the interactive terminal UI")]
    Manage,
}

#[derive(Debug, Args)]
#[command(
    group(ArgGroup::new("target").required(true).args(["name", "tag"])),
    after_help = "Examples:\n  plinks open docs\n  plinks open api\n  plinks open --tag rust"
)]
pub struct OpenArgs {
    #[arg(
        value_name = "NAME_OR_ALIAS",
        help = "Primary name or alias of the saved link to open"
    )]
    pub name: Option<String>,
    #[arg(long, value_name = "TAG", help = "Open every saved link with this tag")]
    pub tag: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, value_name = "TAG", help = "Only show saved links with this tag")]
    pub tag: Option<String>,
}

#[derive(Debug, Args)]
#[command(
    after_help = "Examples:\n  plinks add docs https://docs.rs\n  plinks add docs https://docs.rs --alias api --tag rust --note \"Rust API docs\"\n  plinks add docs https://docs.rs --force"
)]
pub struct AddArgs {
    #[arg(
        value_name = "PRIMARY_NAME",
        help = "Stable primary name for the link, used by `plinks open` and `plinks remove`"
    )]
    pub primary: String,
    #[arg(value_name = "URL", help = "URL to store for this link")]
    pub url: String,
    #[arg(
        long = "alias",
        value_name = "ALIAS",
        help = "Additional name that can also be used with `plinks open`"
    )]
    pub aliases: Vec<String>,
    #[arg(
        long = "tag",
        value_name = "TAG",
        help = "Tag used to group links for `plinks list --tag` or `plinks open --tag`"
    )]
    pub tags: Vec<String>,
    #[arg(long, value_name = "TEXT", help = "Optional human-readable note")]
    pub note: Option<String>,
    #[arg(long, help = "Replace an existing link with the same primary name")]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(
        value_name = "PRIMARY_NAME",
        help = "Primary name of the saved link to remove"
    )]
    pub primary: String,
}
