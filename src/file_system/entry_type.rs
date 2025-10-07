use std::fs::{self, DirEntry};

pub enum EntryType {
    Dir,
    File,
}

impl EntryType {
    pub fn try_match(&self, entry: &DirEntry) -> anyhow::Result<bool> {
        let metadata = fs::metadata(entry.path())?;
        match self {
            Self::Dir => Ok(metadata.is_dir()),
            Self::File => Ok(metadata.is_file()),
        }
    }
}
