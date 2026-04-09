use std::path::{Path, PathBuf};

use anyhow::Result;

pub const CONFIG_FILE_NAME: &str = "project-links.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfigPath {
    pub project_dir: PathBuf,
    pub config_path: PathBuf,
    pub git_root: Option<PathBuf>,
}

pub fn resolve_config_path(cwd: &Path) -> Result<ResolvedConfigPath> {
    let cwd_config = cwd.join(CONFIG_FILE_NAME);
    if cwd_config.exists() {
        return Ok(ResolvedConfigPath {
            project_dir: cwd.to_path_buf(),
            config_path: cwd_config,
            git_root: find_git_root(cwd),
        });
    }

    let git_root = find_git_root(cwd);
    if let Some(root) = git_root.as_deref() {
        for ancestor in cwd.ancestors().skip(1) {
            let candidate = ancestor.join(CONFIG_FILE_NAME);
            if candidate.exists() {
                return Ok(ResolvedConfigPath {
                    project_dir: ancestor.to_path_buf(),
                    config_path: candidate,
                    git_root: git_root.clone(),
                });
            }
            if ancestor == root {
                break;
            }
        }

        return Ok(ResolvedConfigPath {
            project_dir: root.to_path_buf(),
            config_path: root.join(CONFIG_FILE_NAME),
            git_root,
        });
    }

    Ok(ResolvedConfigPath {
        project_dir: cwd.to_path_buf(),
        config_path: cwd.join(CONFIG_FILE_NAME),
        git_root: None,
    })
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn current_directory_config_wins() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::write(
            temp.path().join(CONFIG_FILE_NAME),
            "version = 1\n\n[links]\n",
        )
        .unwrap();

        let resolved = resolve_config_path(temp.path()).unwrap();
        assert_eq!(resolved.project_dir, temp.path());
        assert_eq!(resolved.config_path, temp.path().join(CONFIG_FILE_NAME));
    }

    #[test]
    fn ancestor_config_is_found() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("repo");
        let nested = root.join("a/b/c");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join(CONFIG_FILE_NAME), "version = 1\n\n[links]\n").unwrap();

        let resolved = resolve_config_path(&nested).unwrap();
        assert_eq!(resolved.project_dir, root);
    }

    #[test]
    fn git_root_fallback_is_used_when_no_file_exists() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("repo");
        let nested = root.join("nested");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(&nested).unwrap();

        let resolved = resolve_config_path(&nested).unwrap();
        assert_eq!(resolved.project_dir, root);
        assert_eq!(resolved.config_path, root.join(CONFIG_FILE_NAME));
    }

    #[test]
    fn no_git_root_falls_back_to_cwd() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path().join("plain");
        fs::create_dir_all(&cwd).unwrap();

        let resolved = resolve_config_path(&cwd).unwrap();
        assert_eq!(resolved.project_dir, cwd);
    }
}
