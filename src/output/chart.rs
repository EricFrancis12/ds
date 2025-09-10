use std::collections::HashMap;

use console;
use once_cell::sync::Lazy;

use crate::{file_system::entry::FsEntry, units::system::UnitSystem};

pub fn print_chart(
    entries: &Vec<FsEntry>,
    unit_system: &UnitSystem,
    max_size: u64,
    max_size_digits: usize,
    max_name_len: usize,
    max_bar_width: u32,
) {
    print!(
        "{}",
        make_chart(
            entries,
            unit_system,
            max_size,
            max_size_digits,
            max_name_len,
            max_bar_width,
        )
    );
}

pub fn make_chart(
    entries: &Vec<FsEntry>,
    unit_system: &UnitSystem,
    max_size: u64,
    max_size_digits: usize,
    max_name_len: usize,
    max_bar_width: u32,
) -> String {
    let max_bar_width_f64 = max_bar_width as f64;
    let max_size_f64 = max_size as f64;

    let mut chart = String::new();

    for fse in entries {
        let size = fse.size().unwrap_or(0);

        let mut bar_len = if max_size == 0 {
            0
        } else {
            ((size as f64 / max_size_f64) * max_bar_width_f64).round() as usize
        };
        if size > 0 && bar_len == 0 {
            bar_len = 1;
        }

        let raw_name = fse.name_str();
        let colored_name = match fse {
            FsEntry::File { .. } => raw_name,
            FsEntry::Dir { .. } => &format!("\x1b[34m{}\x1b[0m", raw_name), // Blue,
            FsEntry::Unknown { .. } => &format!("\x1b[31m{}\x1b[0m", raw_name), // Red
        };
        let name = console::pad_str(&colored_name, max_name_len, console::Alignment::Left, None);

        static RIGHT_ALIGNS: Lazy<HashMap<UnitSystem, usize>> = Lazy::new(|| {
            let mut map = HashMap::new();

            map.insert(UnitSystem::Raw, 0);

            let max_len = |units: &[&str]| units.iter().map(|u| u.len()).max().unwrap_or(0);
            map.insert(UnitSystem::SI, max_len(&UnitSystem::SI_UNITS) + 1);
            map.insert(UnitSystem::Binary, max_len(&UnitSystem::BINARY_UNITS) + 1);

            map.insert(UnitSystem::Lines, UnitSystem::LINES.len() + 1);

            map
        });

        chart.push_str(&format!(
            "{name}   [{bar:<bar_width$}]   {size:>size_width$}\n",
            bar = "#".repeat(bar_len),
            bar_width = max_bar_width as usize,
            size = unit_system.format_entry(fse),
            size_width = max_size_digits + *RIGHT_ALIGNS.get(unit_system).unwrap(),
        ));
    }

    chart
}
