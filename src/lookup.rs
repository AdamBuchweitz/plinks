use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::config::{Config, LinkEntry};

pub type AliasIndex = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy)]
pub struct ResolvedLink<'a> {
    pub primary: &'a str,
    pub entry: &'a LinkEntry,
}

pub fn build_alias_index(config: &Config) -> Result<AliasIndex> {
    let mut index = BTreeMap::new();
    for (primary, entry) in &config.links {
        insert(&mut index, primary, primary)?;
        for alias in &entry.aliases {
            insert(&mut index, alias, primary)?;
        }
    }
    Ok(index)
}

pub fn resolve_alias<'a>(config: &'a Config, alias: &str) -> Result<Option<ResolvedLink<'a>>> {
    let index = build_alias_index(config)?;
    let Some(primary) = index.get(alias) else {
        return Ok(None);
    };
    let (primary, entry) = config
        .links
        .get_key_value(primary)
        .expect("alias index should only reference existing primaries");
    Ok(Some(ResolvedLink { primary, entry }))
}

pub fn links_for_tag<'a>(config: &'a Config, tag: &str) -> Vec<ResolvedLink<'a>> {
    config
        .links
        .iter()
        .filter(|(_, entry)| entry.tags.iter().any(|candidate| candidate == tag))
        .map(|(primary, entry)| ResolvedLink { primary, entry })
        .collect()
}

fn insert(index: &mut AliasIndex, alias: &str, primary: &str) -> Result<()> {
    if let Some(existing) = index.insert(alias.to_string(), primary.to_string()) {
        bail!("alias '{alias}' is already used by primary alias '{existing}'");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CandidateLink, Config};

    #[test]
    fn resolves_primary_alias() {
        let mut config = Config::default();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "docs".into(),
                    "https://docs.rs".into(),
                    vec![],
                    vec![],
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let resolved = resolve_alias(&config, "docs").unwrap().unwrap();
        assert_eq!(resolved.primary, "docs");
    }

    #[test]
    fn resolves_extra_alias() {
        let mut config = Config::default();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "docs".into(),
                    "https://docs.rs".into(),
                    vec!["rust".into()],
                    vec![],
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let resolved = resolve_alias(&config, "rust").unwrap().unwrap();
        assert_eq!(resolved.primary, "docs");
    }

    #[test]
    fn exact_matching_only() {
        let mut config = Config::default();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "docs".into(),
                    "https://docs.rs".into(),
                    vec![],
                    vec![],
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        assert!(resolve_alias(&config, "doc").unwrap().is_none());
    }

    #[test]
    fn tag_filter_returns_sorted_results() {
        let mut config = Config::default();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "b".into(),
                    "https://b.example".into(),
                    vec![],
                    vec!["shared".into()],
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "a".into(),
                    "https://a.example".into(),
                    vec![],
                    vec!["shared".into()],
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let primaries = links_for_tag(&config, "shared")
            .into_iter()
            .map(|link| link.primary.to_string())
            .collect::<Vec<_>>();
        assert_eq!(primaries, vec!["a", "b"]);
    }
}
