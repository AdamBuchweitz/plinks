use clap::{ArgGroup, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "plinks", about = "Project-local link manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Open(OpenArgs),
    #[command(alias = "ls")]
    List(ListArgs),
    Add(AddArgs),
    Remove(RemoveArgs),
    Manage,
}

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("target").required(true).args(["name", "tag"])))]
pub struct OpenArgs {
    pub name: Option<String>,
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    pub primary: String,
    pub url: String,
    #[arg(long = "alias")]
    pub aliases: Vec<String>,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    pub primary: String,
}
