use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateLink {
    pub primary: String,
    pub entry: LinkEntry,
}

impl CandidateLink {
    pub fn new(
        primary: String,
        url: String,
        aliases: Vec<String>,
        tags: Vec<String>,
        note: Option<String>,
    ) -> Result<Self> {
        let primary = normalize_primary(&primary)?;
        let url = normalize_url(&url)?;
        let aliases = normalize_aliases(aliases)?;
        if aliases.iter().any(|alias| alias == &primary) {
            bail!("primary alias '{primary}' cannot also appear in aliases");
        }
        let tags = normalize_tags(tags)?;
        let note = normalize_note(note);

        Ok(Self {
            primary,
            entry: LinkEntry {
                url,
                aliases,
                tags,
                note,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub links: BTreeMap<String, LinkEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            links: BTreeMap::new(),
        }
    }
}

impl Config {
    pub fn validate_and_normalize(self) -> Result<Self> {
        if self.version != 1 {
            bail!("unsupported config version {}; expected 1", self.version);
        }

        let mut normalized_links = BTreeMap::new();
        for (primary, entry) in self.links {
            let primary = normalize_primary(&primary)?;
            let entry = entry.validate_and_normalize(&primary)?;
            if normalized_links.contains_key(&primary) {
                bail!("primary alias '{primary}' is duplicated");
            }
            normalized_links.insert(primary, entry);
        }

        let normalized = Self {
            version: 1,
            links: normalized_links,
        };
        normalized.validate_namespace()?;
        Ok(normalized)
    }

    pub fn validate_namespace(&self) -> Result<()> {
        let mut owners = BTreeMap::<String, String>::new();
        for (primary, entry) in &self.links {
            insert_unique_alias(&mut owners, primary, primary)?;
            for alias in &entry.aliases {
                insert_unique_alias(&mut owners, alias, primary)?;
            }
        }
        Ok(())
    }

    pub fn save_link(
        &mut self,
        original_primary: Option<&str>,
        candidate: CandidateLink,
    ) -> Result<()> {
        let mut next = self.clone();
        if let Some(original_primary) = original_primary {
            let original_primary = normalize_primary(original_primary)?;
            next.links
                .remove(&original_primary)
                .with_context(|| format!("primary alias '{original_primary}' does not exist"))?;
        }

        if next.links.contains_key(&candidate.primary) {
            bail!("primary alias '{}' already exists", candidate.primary);
        }

        next.links
            .insert(candidate.primary.clone(), candidate.entry.clone());
        next.validate_namespace()?;
        *self = next;
        Ok(())
    }

    pub fn canonical_toml(&self) -> String {
        let mut out = String::from("version = 1\n\n[links]\n");
        for (primary, entry) in &self.links {
            out.push('\n');
            out.push_str(&format!("[links.{primary}]\n"));
            out.push_str("url = ");
            out.push_str(&toml_string(&entry.url));
            out.push('\n');
            if !entry.aliases.is_empty() {
                out.push_str("aliases = ");
                out.push_str(&toml_array(&entry.aliases));
                out.push('\n');
            }
            if !entry.tags.is_empty() {
                out.push_str("tags = ");
                out.push_str(&toml_array(&entry.tags));
                out.push('\n');
            }
            if let Some(note) = &entry.note {
                out.push_str("note = ");
                out.push_str(&toml_string(note));
                out.push('\n');
            }
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LinkEntry {
    pub url: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl LinkEntry {
    pub fn validate_and_normalize(mut self, primary: &str) -> Result<Self> {
        self.url = normalize_url(&self.url)?;
        self.aliases = normalize_aliases(self.aliases)?;
        if self.aliases.iter().any(|alias| alias == primary) {
            bail!("primary alias '{primary}' cannot also appear in aliases");
        }
        self.tags = normalize_tags(self.tags)?;
        self.note = normalize_note(self.note);
        Ok(self)
    }
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: Config,
    pub raw: String,
}

pub fn load_existing(path: &Path) -> Result<Option<LoadedConfig>> {
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let parsed = toml::from_str::<Config>(&raw)
        .with_context(|| format!("failed to parse config {}", path.display()))?;
    let config = parsed.validate_and_normalize()?;
    Ok(Some(LoadedConfig { config, raw }))
}

pub fn write_config(path: &Path, config: &Config) -> Result<String> {
    let normalized = config.clone().validate_and_normalize()?;
    let text = normalized.canonical_toml();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, &text).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(text)
}

pub fn normalize_primary(value: &str) -> Result<String> {
    normalize_name("primary alias", value)
}

pub fn normalize_alias(value: &str) -> Result<String> {
    normalize_name("alias", value)
}

pub fn normalize_tag(value: &str) -> Result<String> {
    normalize_name("tag", value)
}

fn normalize_name(kind: &str, value: &str) -> Result<String> {
    let normalized = value.trim().to_ascii_lowercase();
    validate_name(kind, &normalized)?;
    Ok(normalized)
}

fn validate_name(kind: &str, value: &str) -> Result<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        bail!("{kind} cannot be empty");
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        bail!("{kind} '{value}' must start with a letter or number");
    }
    for ch in chars {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_' && ch != '-' {
            bail!("{kind} '{value}' contains invalid character '{ch}'");
        }
    }
    Ok(())
}

fn normalize_aliases(values: Vec<String>) -> Result<Vec<String>> {
    let mut aliases = values
        .into_iter()
        .map(|value| normalize_alias(&value))
        .collect::<Result<Vec<_>>>()?;
    aliases.sort();
    for pair in aliases.windows(2) {
        if pair[0] == pair[1] {
            bail!("duplicate alias '{}'", pair[0]);
        }
    }
    Ok(aliases)
}

fn normalize_tags(values: Vec<String>) -> Result<Vec<String>> {
    let mut tags = values
        .into_iter()
        .map(|value| normalize_tag(&value))
        .collect::<Result<Vec<_>>>()?;
    tags.sort();
    tags.dedup();
    Ok(tags)
}

fn normalize_note(note: Option<String>) -> Option<String> {
    note.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_url(value: &str) -> Result<String> {
    let normalized = value.trim().to_string();
    let parsed = Url::parse(&normalized).with_context(|| format!("invalid URL '{normalized}'"))?;
    if parsed.scheme().is_empty() {
        bail!("invalid URL '{normalized}': missing scheme");
    }
    Ok(normalized)
}

fn insert_unique_alias(
    owners: &mut BTreeMap<String, String>,
    alias: &str,
    primary: &str,
) -> Result<()> {
    if let Some(existing) = owners.insert(alias.to_string(), primary.to_string()) {
        bail!("alias '{alias}' is already used by primary alias '{existing}'");
    }
    Ok(())
}

fn default_version() -> u32 {
    1
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}

fn toml_array(values: &[String]) -> String {
    toml::Value::Array(
        values
            .iter()
            .cloned()
            .map(toml::Value::String)
            .collect::<Vec<_>>(),
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let parsed = toml::from_str::<Config>(
            r#"
            version = 1

            [links.docs]
            url = "https://docs.rs"
            "#,
        )
        .unwrap()
        .validate_and_normalize()
        .unwrap();

        assert_eq!(parsed.links["docs"].url, "https://docs.rs");
        assert!(parsed.links["docs"].aliases.is_empty());
    }

    #[test]
    fn parse_full_config() {
        let parsed = toml::from_str::<Config>(
            r#"
            version = 1

            [links.db]
            url = "postgres://localhost:5432/app"
            aliases = ["database", "postgres"]
            tags = ["backend", "local"]
            note = "Local development database"
            "#,
        )
        .unwrap()
        .validate_and_normalize()
        .unwrap();

        let entry = &parsed.links["db"];
        assert_eq!(entry.aliases, vec!["database", "postgres"]);
        assert_eq!(entry.tags, vec!["backend", "local"]);
        assert_eq!(entry.note.as_deref(), Some("Local development database"));
    }

    #[test]
    fn canonical_write_sorts_links_aliases_and_tags() {
        let mut config = Config::default();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "b".into(),
                    "https://b.example".into(),
                    vec!["z".into(), "alt".into()],
                    vec!["two".into(), "one".into()],
                    Some(" note ".into()),
                )
                .unwrap(),
            )
            .unwrap();
        config
            .save_link(
                None,
                CandidateLink::new("a".into(), "https://a.example".into(), vec![], vec![], None)
                    .unwrap(),
            )
            .unwrap();

        let text = config.canonical_toml();
        let expected = r#"version = 1

[links]

[links.a]
url = "https://a.example"

[links.b]
url = "https://b.example"
aliases = ["alt", "z"]
tags = ["one", "two"]
note = "note"
"#;
        assert_eq!(text, expected);
    }

    #[test]
    fn duplicate_aliases_across_entries_are_rejected() {
        let config = Config {
            version: 1,
            links: BTreeMap::from([
                (
                    "one".into(),
                    LinkEntry {
                        url: "https://one.example".into(),
                        aliases: vec!["shared".into()],
                        tags: vec![],
                        note: None,
                    },
                ),
                (
                    "two".into(),
                    LinkEntry {
                        url: "https://two.example".into(),
                        aliases: vec!["shared".into()],
                        tags: vec![],
                        note: None,
                    },
                ),
            ]),
        };

        assert!(config.validate_and_normalize().is_err());
    }

    #[test]
    fn extra_alias_matching_primary_is_rejected() {
        let config = Config {
            version: 1,
            links: BTreeMap::from([(
                "db".into(),
                LinkEntry {
                    url: "https://example.com".into(),
                    aliases: vec!["db".into()],
                    tags: vec![],
                    note: None,
                },
            )]),
        };

        assert!(config.validate_and_normalize().is_err());
    }

    #[test]
    fn duplicate_primaries_after_normalization_are_rejected() {
        let config = Config {
            version: 1,
            links: BTreeMap::from([
                (
                    "Docs".into(),
                    LinkEntry {
                        url: "https://docs.rs".into(),
                        aliases: vec![],
                        tags: vec![],
                        note: None,
                    },
                ),
                (
                    "docs".into(),
                    LinkEntry {
                        url: "https://example.com".into(),
                        aliases: vec![],
                        tags: vec![],
                        note: None,
                    },
                ),
            ]),
        };

        assert!(config.validate_and_normalize().is_err());
    }

    #[test]
    fn invalid_alias_and_tag_are_rejected() {
        assert!(normalize_alias("Bad Alias").is_err());
        assert!(normalize_tag("*bad").is_err());
    }

    #[test]
    fn whitespace_note_becomes_none() {
        let candidate = CandidateLink::new(
            "docs".into(),
            "https://docs.rs".into(),
            vec![],
            vec![],
            Some("   ".into()),
        )
        .unwrap();

        assert_eq!(candidate.entry.note, None);
    }

    #[test]
    fn invalid_url_is_rejected() {
        assert!(
            CandidateLink::new("docs".into(), "not a url".into(), vec![], vec![], None).is_err()
        );
    }

    #[test]
    fn custom_scheme_url_is_accepted() {
        let candidate = CandidateLink::new(
            "editor".into(),
            "vscode://file/tmp/test".into(),
            vec![],
            vec![],
            None,
        )
        .unwrap();

        assert_eq!(candidate.entry.url, "vscode://file/tmp/test");
    }
}
