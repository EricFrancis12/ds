use crate::{
    bytes::units::ByteUnitSystem,
    file_system::entry::{FsEntry, UNKNOWN_ENTRY},
};

pub fn make_chart(
    entries: &Vec<FsEntry>,
    bus: &ByteUnitSystem,
    max_size: u64,
    max_size_digits: usize,
    max_name_len: usize,
    max_bar_width: u32,
) -> String {
    let max_bar_width_f64: f64 = max_bar_width as f64;
    let max_size_f64 = max_size as f64;

    let mut chart = String::new();

    for fse in entries {
        let mut bar_len = if max_size == 0 {
            0
        } else {
            ((fse.size as f64 / max_size_f64) * max_bar_width_f64).round() as usize
        };

        if fse.size > 0 && bar_len == 0 {
            bar_len = 1;
        }

        let bar = "#".repeat(bar_len);

        let raw_name = match &fse.name {
            Some(s) => s,
            None => UNKNOWN_ENTRY,
        };

        let colored_name = match (fse.name.is_some(), fse.is_dir) {
            (true, Some(true)) => &format!("\x1b[34m{}\x1b[0m", raw_name), // Blue
            (true, Some(false)) => raw_name,
            _ => &format!("\x1b[31m{}\x1b[0m", raw_name), // Red
        };

        let name = console::pad_str(colored_name, max_name_len, console::Alignment::Left, None);

        chart.push_str(&format!(
            "{name}   [{:<width_bar$}]   {:>width_size$}\n",
            bar,
            bus.format(fse.size),
            name = name,
            width_bar = max_bar_width as usize,
            width_size = max_size_digits
        ));
    }

    chart
}

pub fn print_chart(
    entries: &Vec<FsEntry>,
    bus: &ByteUnitSystem,
    max_size: u64,
    max_size_digits: usize,
    max_name_len: usize,
    max_bar_width: u32,
) {
    print!(
        "{}",
        make_chart(
            entries,
            bus,
            max_size,
            max_size_digits,
            max_name_len,
            max_bar_width
        )
    );
}
