use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAX_BAR_WIDTH: usize = 50;
const MAX_BAR_WIDTH_F64: f64 = MAX_BAR_WIDTH as f64;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(default_value = ".")]
    dir: String,
    #[arg(short = 'n', long = "name", conflicts_with = "size")]
    sort_by_name: bool,
    #[arg(short = 's', long = "size", conflicts_with = "name")]
    sort_by_size: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let target_path = Path::new(&args.dir);

    if !target_path.exists() || !target_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{}' is not a valid directory.", args.dir),
        ));
    }

    let entries: Vec<_> = fs::read_dir(target_path)?.filter_map(Result::ok).collect();

    if entries.is_empty() {
        println!("No files or directories found in '{}'.", args.dir);
        return Ok(());
    }

    let (results, errors): (Vec<_>, Vec<_>) = entries
        .into_par_iter()
        .map(|entry| {
            let path: PathBuf = entry.path();
            let name = entry.file_name().into_string().unwrap_or_default();

            match get_size(&path) {
                Ok(size) => Ok((name, size)),
                Err(e) => Err((name, e)),
            }
        })
        .partition(Result::is_ok);

    let mut results: Vec<(String, u64)> = results.into_iter().filter_map(Result::ok).collect();
    let errors: Vec<(String, io::Error)> = errors.into_iter().filter_map(Result::err).collect();

    if !errors.is_empty() {
        eprintln!("\nSome errors occurred:");
        for (name, err) in &errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    if results.is_empty() {
        println!("\nAll entries failed. Nothing to show.");
        return Ok(());
    }

    if args.sort_by_name {
        results.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    } else if args.sort_by_size {
        results.sort_by(|a, b| b.1.cmp(&a.1));
    }

    let max_name_len = results
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);
    let max_size = results.iter().map(|(_, size)| *size).max().unwrap_or(1);
    let max_size_digits = results
        .iter()
        .map(|(_, size)| size.to_string().len())
        .max()
        .unwrap_or(1);

    println!("\nFile/Directory Sizes in '{}'", args.dir);
    println!("==============================================");

    let max_size_f64 = max_size as f64;

    for (name, size) in results {
        let mut bar_len = if max_size == 0 {
            0
        } else {
            (size as f64 / max_size_f64 * MAX_BAR_WIDTH_F64).round() as usize
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
