use std::cmp::Ordering;

use crate::config::SortBy;

pub struct FsEntry {
    pub name: Option<String>,
    pub is_dir: Option<bool>,
    pub size: u64,
    pub lines: Option<u64>,
}

impl FsEntry {
    pub const UNKNOWN_ENTRY: &str = "[Unknown Entry]";

    pub fn get_name(&self) -> &str {
        match self.name.as_ref() {
            Some(name) => name,
            None => Self::UNKNOWN_ENTRY,
        }
    }
}

pub fn sort_entries(entries: &mut [FsEntry], sort_by: &SortBy, reverse: bool) {
    let compare: fn(&FsEntry, &FsEntry) -> Ordering = match sort_by {
        SortBy::Name => |a, b| match (&a.name, &b.name) {
            (Some(a_name), Some(b_name)) => {
                a_name.to_lowercase().cmp(&b_name.to_lowercase()).reverse()
            }
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        },
        SortBy::Size => |a, b| b.size.cmp(&a.size),
        SortBy::Type => |a, b| {
            let cmp_val = |is_dir: Option<bool>| match is_dir {
                Some(true) => 0,
                Some(false) => 1,
                None => 2,
            };
            cmp_val(a.is_dir).cmp(&cmp_val(b.is_dir))
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
