use anyhow::anyhow;
use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;

use crate::{
    config::{Config, SortBy},
    file_system::entry_type::EntryType,
    filter::DirEntryFilter,
    units::system::UnitSystem,
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(default_value = ".", help = "Root directory to scan")]
    pub dir: String,

    #[arg(
        name = "name",
        long = "name",
        short = 'n',
        conflicts_with_all = ["size", "type"],
        help = "Sort entries by name"
    )]
    pub sort_by_name: bool,

    #[arg(
        name = "size",
        long = "size",
        short = 's',
        conflicts_with_all = ["name", "type"],
        help = "Sort entries by size"
    )]
    pub sort_by_size: bool,

    #[arg(
        name = "type",
        long = "type",
        short = 't',
        conflicts_with_all = ["name", "size"],
        help = "Sort entries by type"
    )]
    pub sort_by_type: bool,

    #[arg(
        name = "reverse",
        long = "reverse",
        aliases = ["rev", "reversed"],
        help = "Reverse the sorting order"
    )]
    pub reverse: bool,

    #[arg(
        name = "si",
        long = "si",
        conflicts_with = "binary",
        help = "Use SI (decimal) units for sizes (e.g., KB, MB)"
    )]
    pub si: bool,

    #[arg(
        name = "binary",
        long = "binary",
        alias = "bin",
        conflicts_with = "si",
        help = "Use binary (IEC) units for sizes (e.g., KiB, MiB)"
    )]
    pub binary: bool,

    #[arg(
        name = "regex",
        long = "regex",
        short = 'r',
        help = "Filter file names using a regular expression"
    )]
    pub regex: Option<String>,

    #[arg(
        name = "include",
        long = "include",
        short = 'i',
        conflicts_with = "regex",
        help = "Include only entries matching these glob patterns"
    )]
    pub include: Vec<String>,

    #[arg(
        name = "exclude",
        long = "exclude",
        short = 'e',
        conflicts_with = "regex",
        help = "Exclude entries matching these glob patterns"
    )]
    pub exclude: Vec<String>,

    #[arg(
        name = "dirs-only",
        long = "dirs-only",
        alias = "dirs",
        conflicts_with = "files-only",
        help = "Show directories only"
    )]
    pub dirs_only: bool,

    #[arg(
        name = "files-only",
        long = "files-only",
        alias = "files",
        conflicts_with = "dirs-only",
        help = "Show files only"
    )]
    pub files_only: bool,

    #[arg(
        name = "min-size",
        long = "min-size",
        alias = "min",
        help = "Minimum file size to include (in bytes)"
    )]
    pub min_size: Option<u64>,

    #[arg(
        name = "max-size",
        long = "max-size",
        alias = "max",
        help = "Maximum file size to include (in bytes)"
    )]
    pub max_size: Option<u64>,

    #[arg(
        name = "max-bar-width",
        long = "max-bar-width",
        aliases = ["bw", "bl", "bs"],
        default_value_t = 50,
        help = "Maximum width of visual bar (e.g., for size graphing)"
    )]
    pub max_bar_width: u32,

    #[arg(
        name = "no-errors",
        long = "no-errors",
        aliases = [
            "no-error",
            "no-errs",
            "no-err",
            "noerrors",
            "noerror",
            "noerrs",
            "noerr"
        ],
        help = "Suppress error messages like 'permission denied'"
    )]
    pub no_errors: bool,
}

impl TryInto<Config> for Args {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        let actual_min = self.min_size.unwrap_or(0);
        let actual_max = self.max_size.unwrap_or(u64::MAX);
        if !(actual_min < actual_max) {
            return Err(anyhow!(
                "min_size must be less than max_size (got min_size: {}, max_size: {})",
                actual_min,
                actual_max
            ));
        }

        let unit_system = if self.binary {
            UnitSystem::Binary
        } else if self.si {
            UnitSystem::SI
        } else {
            UnitSystem::Raw
        };

        let sort_by = if self.sort_by_name {
            Some(SortBy::Name)
        } else if self.sort_by_size {
            Some(SortBy::Size)
        } else if self.sort_by_type {
            Some(SortBy::Type)
        } else {
            None
        };

        let filter = if let Some(regex_pattern) = self.regex {
            let re = Regex::new(&regex_pattern)?;
            Some(DirEntryFilter::Regex(re))
        } else if !self.include.is_empty() || !self.exclude.is_empty() {
            Some(DirEntryFilter::Glob {
                include: make_globset(&self.include)?,
                exclude: make_globset(&self.exclude)?,
            })
        } else {
            None
        };

        let needs_type = if self.dirs_only {
            Some(EntryType::Dir)
        } else if self.files_only {
            Some(EntryType::File)
        } else {
            None
        };

        Ok(Config {
            dir: self.dir,
            unit_system,
            sort_by,
            filter,
            reverse: self.reverse,
            needs_type,
            min_size: self.min_size,
            max_size: self.max_size,
            max_bar_width: self.max_bar_width,
            no_errors: self.no_errors,
        })
    }
}

fn make_globset(patterns: &Vec<String>) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    for s in patterns {
        builder.add(Glob::new(&s)?);
    }
    builder.build()
}
