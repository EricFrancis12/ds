use clap::{builder::PossibleValue, ValueEnum};

#[derive(Clone, Debug)]
pub enum ByteUnitSystem {
    Raw,
    SI,
    Binary,
}

impl Default for ByteUnitSystem {
    fn default() -> Self {
        Self::Raw
    }
}

impl ValueEnum for ByteUnitSystem {
    fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
        let input = if ignore_case {
            input.to_lowercase()
        } else {
            input.to_owned()
        };
        match input.as_str() {
            "" | "raw" => Ok(Self::Raw),
            "si" | "1000" => Ok(Self::SI),
            "binary" | "bin" | "1024" => Ok(Self::Binary),
            s => Err(s.to_owned()),
        }
    }

    fn value_variants<'a>() -> &'a [Self] {
        static VARIANTS: [ByteUnitSystem; 3] = [
            ByteUnitSystem::Raw,
            ByteUnitSystem::SI,
            ByteUnitSystem::Binary,
        ];
        &VARIANTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            ByteUnitSystem::Raw => PossibleValue::new("raw").help("Raw bytes with no scaling"),
            ByteUnitSystem::SI => {
                // TODO: define aliases for use here and in from_str
                PossibleValue::new("si").aliases(["1000"]).help(format!(
                    "SI units (base 1000): {}",
                    Self::SI_UNITS.join(", ")
                ), /* TODO: format at compile time */)
            }
            ByteUnitSystem::Binary => {
                // TODO: define aliases for use here and in from_str
                PossibleValue::new("binary").aliases(["bin", "1024"]).help(
                    format!(
                        "Binary units (base 1024): {}",
                        Self::BINARY_UNITS.join(", ")
                    ), /* TODO: format at compile time */
                )
            }
        })
    }
}

impl ByteUnitSystem {
    const SI_UNITS: [&str; 7] = ["B", "kB", "MB", "GB", "TB", "PB", "EB"];
    const BINARY_UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

    pub fn format(&self, bytes: u64) -> String {
        match self {
            ByteUnitSystem::Raw => format!("{}", bytes),
            ByteUnitSystem::SI => Self::do_format(bytes, 1000, Self::SI_UNITS),
            ByteUnitSystem::Binary => Self::do_format(bytes, 1024, Self::BINARY_UNITS),
        }
    }

    fn do_format(bytes: u64, base: u32, units: [&str; 7]) -> String {
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

        format!("{:.2} {}", value, unit)
    }
}
