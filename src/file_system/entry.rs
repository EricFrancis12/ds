use std::{cmp::Ordering, ffi::OsString};

use crate::config::SortBy;

pub enum FsEntry {
    File {
        name: OsString,
        size: u64,
        lines: Option<u64>,
    },
    Dir {
        name: OsString,
        size: u64,
        children: Option<Vec<FsEntry>>,
    },
    Unknown {
        name: OsString,
        size: Option<u64>,
    },
}

impl FsEntry {
    pub const UNKNOWN_ENTRY: &str = "[Unknown Entry]";

    pub fn name(&self) -> &OsString {
        match self {
            Self::File { name, .. } | Self::Dir { name, .. } | Self::Unknown { name, .. } => name,
        }
    }

    pub fn name_str(&self) -> &str {
        self.name()
            .as_os_str()
            .to_str()
            .unwrap_or(Self::UNKNOWN_ENTRY)
    }

    pub fn size(&self) -> Option<u64> {
        match self {
            Self::File { size, .. } | Self::Dir { size, .. } => Some(*size),
            Self::Unknown { size, .. } => *size,
        }
    }

    pub fn lines(&self) -> Option<u64> {
        match self {
            FsEntry::File { lines, .. } => *lines,
            _ => None,
        }
    }
}

pub fn sort_entries(entries: &mut [FsEntry], sort_by: &SortBy, reverse: bool) {
    let compare: fn(&FsEntry, &FsEntry) -> Ordering = match sort_by {
        SortBy::Name => |a, b| {
            a.name_str()
                .to_lowercase()
                .cmp(&b.name_str().to_lowercase())
        },
        SortBy::Size => |a, b| b.size().cmp(&a.size()),
        SortBy::Type => |a, b| {
            let cmp_val = |fse: &FsEntry| match fse {
                FsEntry::Dir { .. } => 0,
                FsEntry::File { .. } => 1,
                FsEntry::Unknown { .. } => 2,
            };
            cmp_val(a).cmp(&cmp_val(b))
        },
    };

    entries.sort_by(|a, b| {
        let mut ordering = compare(a, b);
        if reverse {
            ordering = ordering.reverse();
        }
        ordering
    });
}
