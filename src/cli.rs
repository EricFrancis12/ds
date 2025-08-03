use clap::Parser;

use crate::bytes::system::ByteUnitSystem;

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
    #[arg(
        name = "byte-unit-system",
        long = "byte-unit-system",
        short = 'b',
        aliases = vec!["bytes", "bus"],
        value_enum,
        default_value_t = ByteUnitSystem::default()
    )]
    pub byte_unit_system: ByteUnitSystem,
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
        default_value = "50"
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
