pub mod cli;
pub mod config;
pub mod lookup;
pub mod open_link;
pub mod project_root;
pub mod tui;

use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use cli::{AddArgs, Cli, Command, ListArgs, OpenArgs, RemoveArgs};
use config::{CandidateLink, Config, load_existing, write_config};
use lookup::{links_for_tag, resolve_alias};
use open_link::LinkOpener;
use project_root::resolve_config_path;

pub fn run(cli: Cli, cwd: &Path, opener: &dyn LinkOpener, out: &mut dyn Write) -> Result<()> {
    match cli.command {
        Command::Open(args) => run_open(args, cwd, opener),
        Command::List(args) => run_list(args, cwd, out),
        Command::Add(args) => run_add(args, cwd, out),
        Command::Remove(args) => run_remove(args, cwd, out),
        Command::Manage => {
            let resolved = resolve_config_path(cwd)?;
            tui::run(resolved, opener)
        }
    }
}

fn run_open(args: OpenArgs, cwd: &Path, opener: &dyn LinkOpener) -> Result<()> {
    let resolved = resolve_config_path(cwd)?;
    let document = load_existing(&resolved.config_path)?
        .with_context(|| format!("no config file found at {}", resolved.config_path.display()))?;

    if let Some(tag) = args.tag {
        let tag = config::normalize_tag(&tag)?;
        let matches = links_for_tag(&document.config, &tag);
        if matches.is_empty() {
            bail!("no links found for tag '{tag}'");
        }

        for link in matches {
            opener.open(&link.entry.url)?;
        }
        return Ok(());
    }

    let name = args.name.context("missing link name")?;
    let name = config::normalize_alias(&name)?;
    let resolved_link = resolve_alias(&document.config, &name)?
        .with_context(|| format!("no link found for alias '{name}'"))?;
    opener.open(&resolved_link.entry.url)
}

fn run_list(args: ListArgs, cwd: &Path, out: &mut dyn Write) -> Result<()> {
    let resolved = resolve_config_path(cwd)?;
    let Some(document) = load_existing(&resolved.config_path)? else {
        writeln!(
            out,
            "No project links found yet. Add one with `plinks add <primary> <url>`."
        )?;
        return Ok(());
    };

    let filter_tag = match args.tag {
        Some(tag) => Some(config::normalize_tag(&tag)?),
        None => None,
    };

    let mut rows = Vec::new();
    for (primary, entry) in &document.config.links {
        if let Some(tag) = &filter_tag {
            if !entry.tags.iter().any(|candidate| candidate == tag) {
                continue;
            }
        }

        rows.push(vec![
            primary.clone(),
            entry.aliases.join(", "),
            entry.tags.join(", "),
            entry.url.clone(),
            entry.note.clone().unwrap_or_default(),
        ]);
    }

    if rows.is_empty() {
        if let Some(tag) = filter_tag {
            writeln!(out, "No links found for tag '{tag}'.")?;
        } else {
            writeln!(out, "No links stored in project-links.toml.")?;
        }
        return Ok(());
    }

    write!(
        out,
        "{}",
        render_table(&["PRIMARY", "ALIASES", "TAGS", "URL", "NOTE"], &rows)
    )?;
    Ok(())
}

fn run_add(args: AddArgs, cwd: &Path, out: &mut dyn Write) -> Result<()> {
    let resolved = resolve_config_path(cwd)?;
    let mut config = match load_existing(&resolved.config_path)? {
        Some(document) => document.config,
        None => Config::default(),
    };

    let primary = config::normalize_primary(&args.primary)?;
    let candidate = CandidateLink::new(
        primary.clone(),
        args.url,
        args.aliases,
        args.tags,
        args.note,
    )?;

    if config.links.contains_key(&primary) && !args.force {
        bail!("primary alias '{primary}' already exists; pass --force to replace it");
    }

    let original_primary = if args.force && config.links.contains_key(&primary) {
        Some(primary.as_str())
    } else {
        None
    };
    config.save_link(original_primary, candidate)?;
    write_config(&resolved.config_path, &config)?;

    writeln!(
        out,
        "Saved link '{primary}' to {}.",
        resolved.config_path.display()
    )?;
    Ok(())
}

fn run_remove(args: RemoveArgs, cwd: &Path, out: &mut dyn Write) -> Result<()> {
    let resolved = resolve_config_path(cwd)?;
    let mut config = load_existing(&resolved.config_path)?
        .with_context(|| format!("no config file found at {}", resolved.config_path.display()))?
        .config;

    let primary = config::normalize_primary(&args.primary)?;
    let removed = config
        .links
        .remove(&primary)
        .with_context(|| format!("primary alias '{primary}' does not exist"))?;
    let _ = removed;

    write_config(&resolved.config_path, &config)?;
    writeln!(out, "Removed link '{primary}'.")?;
    Ok(())
}

fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.len());
        }
    }

    let mut out = String::new();
    write_row(
        &mut out,
        headers.iter().map(|value| value.to_string()).collect(),
        &widths,
    );
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("-+-");
    let _ = writeln!(out, "{separator}");
    for row in rows {
        write_row(&mut out, row.clone(), &widths);
    }
    out
}

fn write_row(out: &mut String, cells: Vec<String>, widths: &[usize]) {
    for (idx, cell) in cells.iter().enumerate() {
        if idx > 0 {
            let _ = write!(out, " | ");
        }
        let _ = write!(out, "{cell:width$}", width = widths[idx]);
    }
    let _ = writeln!(out);
}
