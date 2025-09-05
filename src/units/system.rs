use crate::{file_system::entry::FsEntry, units::*};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UnitSystem {
    Raw,
    SI,
    Binary,
    Lines,
}

impl UnitSystem {
    pub const SI_UNITS: [&str; 7] = [B, KB, MB, GB, TB, PB, EB];
    pub const BINARY_UNITS: [&str; 7] = [B, KIB, MIB, GIB, TIB, PIB, EIB];

    pub fn format(&self, units: u64) -> String {
        match self {
            Self::Raw => format!("{}", units),
            Self::SI => Self::format_bytes(units, 1000, Self::SI_UNITS),
            Self::Binary => Self::format_bytes(units, 1024, Self::BINARY_UNITS),
            Self::Lines => format!("{} lines", units),
        }
    }

    pub fn format_entry(&self, fse: &FsEntry) -> String {
        self.format(match self {
            Self::Raw | Self::SI | Self::Binary => fse.size,
            Self::Lines => fse.lines.unwrap_or(0),
        })
    }

    fn format_bytes(bytes: u64, base: u32, units: [&str; 7]) -> String {
        let mut value = bytes as f64;
        let base = base as f64;
        let mut unit = units[0];

        for &next_unit in &units[1..] {
            if value < base {
                break;
            }
            value /= base;
            unit = next_unit;
        }

        if unit == B {
            format!("{:.0} {}", value, unit)
        } else {
            format!("{:.2} {}", value, unit)
        }
    }
}
