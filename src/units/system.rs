use crate::units::*;

pub enum UnitSystem {
    Raw,
    SI,
    Binary,
}

impl UnitSystem {
    const SI_UNITS: [&str; 7] = [B, KB, MB, GB, TB, PB, EB];
    const BINARY_UNITS: [&str; 7] = [B, KIB, MIB, GIB, TIB, PIB, EIB];

    pub fn format(&self, units: u64) -> String {
        match self {
            Self::Raw => format!("{}", units),
            Self::SI => Self::format_bytes(units, 1000, Self::SI_UNITS),
            Self::Binary => Self::format_bytes(units, 1024, Self::BINARY_UNITS),
        }
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
