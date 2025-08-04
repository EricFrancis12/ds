use clap::Parser;

use crate::{
    bytes::system::ByteUnitSystem,
    config::{Config, SortBy},
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(default_value = ".")]
    pub dir: String,

    #[arg(name = "name", long = "name", short = 'n', conflicts_with_all = vec!["size", "type"])]
    pub sort_by_name: bool,
    #[arg(name = "size", long = "size", short = 's', conflicts_with_all = vec!["name", "type"])]
    pub sort_by_size: bool,
    #[arg(name = "type", long = "type", short = 't', conflicts_with_all = vec!["name", "size"])]
    pub sort_by_type: bool,

    #[arg(name = "si", long = "si", conflicts_with = "binary")]
    pub si: bool,
    #[arg(name = "binary", long = "binary", alias = "bin", conflicts_with = "si")]
    pub binary: bool,

    #[arg(name = "regex", long = "regex", short = 'r')]
    pub regex: Option<String>,
    #[arg(
        name = "include",
        long = "include",
        short = 'i',
        conflicts_with = "regex"
    )]
    pub include: Vec<String>,
    #[arg(
        name = "exclude",
        long = "exclude",
        short = 'e',
        conflicts_with = "regex"
    )]
    pub exclude: Vec<String>,
    #[arg(
        name = "max-bar-width",
        long = "max-bar-width",
        aliases = vec!["bw", "bl", "bs"],
        default_value_t = 50
    )]
    pub max_bar_width: u32,
    #[arg(
        name = "no-errors",
        long = "no-errors",
        aliases = vec![
            "no-error",
            "no-errs",
            "no-err",
            "noerrors",
            "noerror",
            "noerrs",
            "noerr"
        ]
    )]
    pub no_errors: bool,
}

impl Into<Config> for Args {
    fn into(self) -> Config {
        let byte_unit_system = if self.binary {
            ByteUnitSystem::Binary
        } else if self.si {
            ByteUnitSystem::SI
        } else {
            ByteUnitSystem::Raw
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

        Config {
            dir: self.dir,
            byte_unit_system,
            sort_by,
            regex: self.regex,
            include: self.include,
            exclude: self.exclude,
            max_bar_width: self.max_bar_width,
            no_errors: self.no_errors,
        }
    }
}
