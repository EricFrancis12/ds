use std::fs::DirEntry;

use anyhow::anyhow;
use globset::GlobSet;
use regex;

pub enum DirEntryFilter {
    Regex(regex::Regex),
    Glob { include: GlobSet, exclude: GlobSet },
}

impl DirEntryFilter {
    pub fn try_match(&self, entry: &DirEntry) -> anyhow::Result<bool> {
        let name = entry.file_name();
        match self {
            Self::Regex(re) => match name.to_str() {
                Some(s) => Ok(re.is_match(s)),
                None => Err(anyhow!(
                    "cannot convert OsString to &str; skipping regex match"
                )),
            },
            Self::Glob { include, exclude } => {
                let is_match = (include.is_empty() || include.is_match(&name))
                    && (exclude.is_empty() || !exclude.is_match(name));
                Ok(is_match)
            }
        }
    }
}
