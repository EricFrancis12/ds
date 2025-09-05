use std::time::Duration;

use crate::units::system::UnitSystem;

pub fn make_summary(
    dir: impl Into<String>,
    resolved_dir: impl Into<String>,
    unit_system: &UnitSystem,
    total_size: u64,
    total_lines: u64,
    dir_count: usize,
    file_count: usize,
    unknown_count: usize,
    results_len: usize,
    errors_len: usize,
    took: Duration,
) -> String {
    let mut summary = String::new();
    let mut max_len = 0;
    let mut push = |s: &str| {
        if s.len() > max_len {
            max_len = s.len();
        }
        summary.push_str(s);
    };

    push(&format!("File/Directory Sizes in '{}'\n", dir.into()));
    push(&format!("Resolved Path: {}\n", resolved_dir.into()));

    let units = if unit_system == &UnitSystem::Lines {
        total_lines
    } else {
        total_size
    };
    push(&format!("Total Size: {}\n", unit_system.format(units)));

    let mut items = format!(
        "Items: {} ({} dirs, {} files",
        results_len, dir_count, file_count
    );
    if unknown_count > 0 {
        items.push_str(&format!(", {} unknown", unknown_count));
    }
    items.push_str(")\n");
    push(&items);

    push(&format!("Errors: {}\n", errors_len));
    push(&format!("Took: {:.2?}\n", took));

    let sep = "=".repeat(max_len);
    format!("{}\n{}{}\n\n", sep, summary, sep)
}

pub fn print_summary(
    dir: impl Into<String>,
    resolved_dir: impl Into<String>,
    unit_system: &UnitSystem,
    total_size: u64,
    total_lines: u64,
    dir_count: usize,
    file_count: usize,
    unknown_count: usize,
    results_len: usize,
    errors_len: usize,
    took: Duration,
) {
    print!(
        "{}",
        make_summary(
            dir,
            resolved_dir,
            unit_system,
            total_size,
            total_lines,
            dir_count,
            file_count,
            unknown_count,
            results_len,
            errors_len,
            took,
        )
    );
}
