use std::ffi::OsString;

use anyhow::anyhow;
use clap::Parser;

use crate::{bytes::system::ByteUnitSystem, cli::Args, filter::DirEntryFilter};

pub struct Config {
    pub dir: String,
    pub byte_unit_system: ByteUnitSystem,
    pub sort_by: Option<SortBy>,
    pub filter: Option<DirEntryFilter>,
    pub max_bar_width: u32,
    pub no_errors: bool,
}

impl Config {
    pub fn parse<I, T>(itr: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        match Args::try_parse_from(itr) {
            Ok(args) => Ok(args.try_into()?),
            Err(err) => Err(anyhow!("error parsing arguments into Config: {}", err)),
        }
    }
}

#[derive(Clone, Copy)]
pub enum SortBy {
    Name,
    Size,
    Type,
}
