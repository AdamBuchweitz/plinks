use anyhow::{Context, Result};

pub trait LinkOpener {
    fn open(&self, url: &str) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemOpener;

impl LinkOpener for SystemOpener {
    fn open(&self, url: &str) -> Result<()> {
        open::that(url).with_context(|| format!("failed to open '{url}'"))?;
        Ok(())
    }
}
