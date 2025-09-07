use once_cell::sync::Lazy;

use crate::{file_system::entry::FsEntry, units::system::UnitSystem};

pub fn make_chart(
    entries: &Vec<FsEntry>,
    unit_system: &UnitSystem,
    max_size: u64,
    max_size_digits: usize,
    max_name_len: usize,
    max_bar_width: u32,
) -> String {
    let max_bar_width_f64: f64 = max_bar_width as f64;
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

        let bar = "#".repeat(bar_len);

        let raw_name = fse.name_str();

        let colored_name = match fse {
            FsEntry::File { .. } => raw_name,
            FsEntry::Dir { .. } => &format!("\x1b[34m{}\x1b[0m", raw_name), // Blue,
            FsEntry::Unknown { .. } => &format!("\x1b[31m{}\x1b[0m", raw_name), // Red
        };

        let name = console::pad_str(colored_name, max_name_len, console::Alignment::Left, None);

        static UNITS_MAX_LEN: Lazy<usize> = Lazy::new(|| {
            UnitSystem::BINARY_UNITS
                .iter()
                .chain(UnitSystem::SI_UNITS.iter())
                .map(|u| u.len())
                .max()
                .unwrap_or(0)
        });

        let width_size = max_size_digits + *UNITS_MAX_LEN;

        chart.push_str(&format!(
            "{name}   [{:<width_bar$}]   {:>width_size$}\n",
            bar,
            unit_system.format_entry(fse),
            name = name,
            width_bar = max_bar_width as usize,
            width_size = width_size
        ));
    }

    chart
}

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
            max_bar_width
        )
    );
}
