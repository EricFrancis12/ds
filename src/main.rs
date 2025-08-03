use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

use clap::Parser;

const MAX_BAR_WIDTH: usize = 50;
const MAX_BAR_WIDTH_F64: f64 = MAX_BAR_WIDTH as f64;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(default_value = ".")]
    dir: String,
    #[arg(name = "name", long = "name", short = 'n', conflicts_with = "size")]
    sort_by_name: bool,
    #[arg(name = "size", long = "size", short = 's', conflicts_with = "name")]
    sort_by_size: bool,
    #[arg(name = "bytes-readable", long = "bytes-readable", short = 'b')]
    bytes_readable: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let start = Instant::now();
    let target_path = Path::new(&args.dir);

    if !target_path.exists() || !target_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{}' is not a valid directory.", args.dir),
        ));
    }

    let mut total_size = 0;
    let mut max_name_len = 0;
    let mut max_size = 0;
    let mut max_size_digits = 0;

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for result in fs::read_dir(target_path)? {
        if let Ok(entry) = result {
            let name = entry.file_name().into_string().unwrap_or_default();

            let size = get_size(&entry.path()).unwrap_or_else(|err| {
                errors.push(err);
                0
            });

            total_size += size;

            if name.len() > max_name_len {
                max_name_len = name.len();
            }

            if size > max_size {
                max_size = size;
            }

            if size.to_string().len() > max_size_digits {
                max_size_digits = size.to_string().len();
            }

            results.push((name, size));
        } else {
            errors.push(result.unwrap_err());
        }
    }

    if !errors.is_empty() {
        eprintln!("\nSome errors occurred:");
        for err in &errors {
            eprintln!("{}", err);
        }
    }

    if args.sort_by_name {
        results.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    } else if args.sort_by_size {
        results.sort_by(|a, b| b.1.cmp(&a.1));
    }

    let took = start.elapsed();

    println!("\nFile/Directory Sizes in '{}'", args.dir);
    println!("Total Size: {}", fmt_bytes(total_size, args.bytes_readable));
    println!("Took: {:.2?}", took);
    println!("==============================================");

    let max_size_f64 = max_size as f64;

    for (name, size) in results {
        let mut bar_len = if max_size == 0 {
            0
        } else {
            ((size as f64 / max_size_f64) * MAX_BAR_WIDTH_F64).round() as usize
        };

        if size > 0 && bar_len == 0 {
            bar_len = 1;
        }

        let bar = "#".repeat(bar_len);
        println!(
            "{:<width_name$}   [{:<width_bar$}]   {:>width_size$}",
            name,
            bar,
            fmt_bytes(size, args.bytes_readable),
            width_name = max_name_len,
            width_bar = MAX_BAR_WIDTH,
            width_size = max_size_digits
        );
    }

    Ok(())
}

fn get_size(path: &Path) -> io::Result<u64> {
    if path.is_file() {
        fs::metadata(path).map(|m| m.len())
    } else if path.is_dir() {
        let mut size = 0;
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            size += get_size(&entry.path())?;
        }
        Ok(size)
    } else {
        Ok(0)
    }
}

fn fmt_bytes(bytes: u64, readable: bool) -> String {
    if !readable {
        return format!("{}", bytes);
    }

    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{}{}", bytes, UNITS[unit])
    } else {
        format!("{:.2}{}", size, UNITS[unit])
    }
}
