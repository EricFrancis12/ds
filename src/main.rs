use std::fs;
use std::io;
use std::path::Path;

fn get_size(path: &Path) -> u64 {
    if path.is_file() {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else if path.is_dir() {
        let mut size = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                size += get_size(&entry.path());
            }
        }
        size
    } else {
        0
    }
}

fn main() -> io::Result<()> {
    let max_bar_width = 50;

    let mut items = vec![];

    // Collect name and size of each entry in current directory
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().into_string().unwrap_or_default();
        let size = get_size(&path);
        items.push((name, size));
    }

    if items.is_empty() {
        println!("No files or directories found.");
        return Ok(());
    }

    // Compute max name width and max size value
    let max_name_len = items.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    let max_size = items.iter().map(|(_, size)| *size).max().unwrap_or(1);
    let max_size_digits = items.iter().map(|(_, size)| size.to_string().len()).max().unwrap_or(1);

    println!("\nFile/Directory Sizes (Bytes)");
    println!("==============================");

    for (name, size) in items {
        let mut bar_len = if max_size == 0 {
            0
        } else {
            (size as f64 / max_size as f64 * max_bar_width as f64).round() as usize
        };

        if size > 0 && bar_len == 0 {
            bar_len = 1;
        }

        let bar = "#".repeat(bar_len);
        println!(
            "{:<width_name$}   [{:<width_bar$}]   {:>width_size$} bytes",
            name,
            bar,
            size,
            width_name = max_name_len,
            width_bar = max_bar_width,
            width_size = max_size_digits
        );
    }

    Ok(())
}
