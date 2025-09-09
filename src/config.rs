use std::ffi::OsString;

use clap::Parser;

use crate::{
    cli::Args, file_system::entry_type::EntryType, filter::DirEntryFilter,
    units::system::UnitSystem, utils::tree::TreeDepth,
};

pub struct Config {
    pub dir: String,
    pub unit_system: UnitSystem,
    pub sort_by: Option<SortBy>,
    pub reverse: bool,
    pub filter: Option<DirEntryFilter>,
    pub needs_type: Option<EntryType>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub max_bar_width: u32,
    pub children_depth: Option<TreeDepth>,
    pub max_threads: Option<usize>,
    pub no_errors: bool,
}

impl Config {
    pub fn parse<I, T>(itr: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Args::try_parse_from(itr)?.try_into()
    }
}

#[derive(Clone, Copy)]
pub enum SortBy {
    Name,
    Size,
    Type,
}
