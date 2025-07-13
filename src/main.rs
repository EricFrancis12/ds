use rayon::prelude::*;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAX_BAR_WIDTH: usize = 50;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let target_dir = args.get(1).map(String::as_str).unwrap_or(".");

    let target_path = Path::new(target_dir);

    if !target_path.exists() || !target_path.is_dir() {
        eprintln!("Error: '{}' is not a valid directory.", target_dir);
        std::process::exit(1);
    }

    let entries: Vec<_> = fs::read_dir(target_path)?.filter_map(Result::ok).collect();

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

    let results: Vec<(String, u64)> = results.into_iter().filter_map(Result::ok).collect();
    let errors: Vec<(String, io::Error)> = errors.into_iter().filter_map(Result::err).collect();

    if results.is_empty() && errors.is_empty() {
        println!("No files or directories found in '{}'.", target_dir);
        return Ok(());
    }

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

    println!("\nFile/Directory Sizes in '{}'", target_dir);
    println!("==============================================");

    let max_size_f64 = max_size as f64;
    let max_bar_width_f64 = MAX_BAR_WIDTH as f64;

    for (name, size) in results {
        let mut bar_len = if max_size == 0 {
            0
        } else {
            (size as f64 / max_size_f64 * max_bar_width_f64).round() as usize
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
