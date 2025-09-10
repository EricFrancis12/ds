use std::collections::HashMap;

use console;
use once_cell::sync::Lazy;

use crate::{file_system::entry::FsEntry, units::system::UnitSystem, utils::tree::TreeDepth};

pub fn print_chart(
    entries: &Vec<FsEntry>,
    unit_system: &UnitSystem,
    children_depth: Option<TreeDepth>,
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
            children_depth,
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
    children_depth: Option<TreeDepth>,
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

        let colored_name = fse.name_str_colored();
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

        if let Some(children_depth) = children_depth {
            if let FsEntry::Dir { children, .. } = fse {
                if let Some(children) = children {
                    chart.push_str(&make_children(&children, children_depth, 1));
                }
            }
        }
    }

    chart
}

fn make_children(children: &[FsEntry], children_depth: TreeDepth, curr_depth: usize) -> String {
    let mut s = String::new();
    if children.is_empty() {
        return s;
    }

    let reached_depth = curr_depth >= children_depth;
    let last_child_index = children.len() - 1;

    for (i, child_fse) in children.iter().enumerate() {
        let children = if let FsEntry::Dir { children, .. } = child_fse {
            children.as_ref()
        } else {
            None
        };

        let is_last = i >= last_child_index;
        let symbol = if !is_last && (reached_depth || children.is_none()) {
            "├"
        } else {
            "└"
        };

        const INDENT_SPACES: usize = 2;

        s.push_str(&format!(
            "{:indent$}{symbol} {name}\n",
            "",
            indent = curr_depth * INDENT_SPACES,
            name = child_fse.name_str_colored(),
        ));
        if let Some(children) = children {
            s.push_str(&make_children(children, children_depth, curr_depth + 1));
        }
    }

    s
}
