mod bytes;
mod cli;
mod config;
mod entry;
mod filter;

use std::{
    env,
    fs::{self, DirEntry},
    io::{self, Write},
    path::Path,
    sync::mpsc,
    thread,
    time::Instant,
};

use anyhow::anyhow;
use crossterm::{
    cursor::MoveToColumn,
    terminal::{Clear, ClearType},
};
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    config::{Config, SortBy},
    entry::FsEntry,
};

fn main() -> anyhow::Result<()> {
    let config = Config::parse(env::args())?;

    let start = Instant::now();
    let target_path = Path::new(&config.dir);

    if !target_path.exists() || !target_path.is_dir() {
        return Err(anyhow!("'{}' is not a valid directory.", config.dir));
    }

    let mut errors: Vec<anyhow::Error> = Vec::new();

    let entries: Vec<DirEntry> = fs::read_dir(target_path)?
        .filter_map(|result| match result {
            Ok(entry) => match &config.filter {
                Some(filter) => match filter.try_match(&entry) {
                    Ok(true) => Some(entry),
                    Ok(false) => None,
                    Err(err) => {
                        errors.push(err);
                        None
                    }
                },
                None => Some(entry),
            },
            Err(err) => {
                errors.push(anyhow!("error reading dir entry: {}", err));
                None
            }
        })
        .collect();

    let mut total_size = 0;
    let mut max_name_len = 0;
    let mut max_size = 0;
    let mut max_size_digits = 0;
    let mut results = Vec::new();

    if !entries.is_empty() {
        let pb = ProgressBar::new(entries.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner} Searching... [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len}",
                )
                .unwrap()
                .progress_chars("█░ "),
        );

        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::new();

        const UNKNOWN_ENTRY: &str = "[Unknown Entry]";

        for entry in entries {
            let tx = tx.clone();
            let handle = thread::spawn(move || {
                let mut errs = Vec::new();
                let name = entry.file_name().into_string().unwrap_or_else(|_| {
                    errs.push(anyhow!(
                        "error getting entry name (entry will be named {} in results)",
                        UNKNOWN_ENTRY
                    ));
                    String::from(UNKNOWN_ENTRY)
                });

                let size = get_size(&entry.path()).unwrap_or_else(|err| {
                    errs.push(anyhow!("error getting size for '{}': {}", name, err));
                    0
                });

                let is_dir = match entry.metadata() {
                    Ok(m) => Some(m.is_dir()),
                    Err(err) => {
                        errs.push(anyhow!("error getting metadata for '{}': {}", name, err));
                        None
                    }
                };

                let fse = FsEntry { name, size, is_dir };
                tx.send((fse, errs)).expect("Failed to send");
            });

            handles.push(handle);
        }

        drop(tx);

        for (fse, errs) in rx {
            total_size += fse.size;

            if fse.name.len() > max_name_len {
                max_name_len = fse.name.len();
            }

            if fse.size > max_size {
                max_size = fse.size;
            }

            if fse.size.to_string().len() > max_size_digits {
                max_size_digits = fse.size.to_string().len();
            }

            results.push(fse);
            for err in errs {
                errors.push(err);
            }

            pb.inc(1);
        }

        for handle in handles {
            let _ = handle.join();
        }

        pb.finish_and_clear();

        if let Some(sort_by) = config.sort_by {
            let mut stderr = io::stderr();

            write!(stderr, "Sorting {} results...", results.len()).unwrap();
            stderr.flush().unwrap();

            match sort_by {
                SortBy::Name => {
                    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                }
                SortBy::Size => results.sort_by(|a, b| b.size.cmp(&a.size)),
                SortBy::Type => results.sort_by(|a, b| {
                    let cmp_val = |is_dir: Option<bool>| match is_dir {
                        Some(true) => 0,
                        Some(false) => 1,
                        None => 2,
                    };
                    cmp_val(a.is_dir).cmp(&cmp_val(b.is_dir))
                }),
            }

            crossterm::execute!(stderr, MoveToColumn(0), Clear(ClearType::CurrentLine)).unwrap();
        }
    }

    let resolved_dir: &str = match fs::canonicalize(Path::new(&config.dir)) {
        Ok(path) => &format!("{}", path.to_str().unwrap_or(&config.dir)),
        Err(err) => {
            errors.push(anyhow!(
                "error resolving full path for '{}': {}",
                config.dir,
                err
            ));
            &config.dir
        }
    };

    let took = start.elapsed();

    if !config.no_errors && !errors.is_empty() {
        eprintln!("\n=== START ERRORS ===");
        for err in &errors {
            eprintln!("{}", err);
        }
        eprintln!("=== END ERRORS ===\n");
    }

    let mut summary = String::new();
    let mut max_len = 0;
    let mut push = |s: &str| {
        if s.len() > max_len {
            max_len = s.len();
        }
        summary.push_str(s);
    };

    push(&format!("File/Directory Sizes in '{}'\n", config.dir));
    push(&format!("Resolved Path: {}\n", resolved_dir));
    push(&format!(
        "Total Size: {}\n",
        config.byte_unit_system.format(total_size)
    ));
    push(&format!("Items: {}\n", results.len()));
    push(&format!("Errors: {}\n", errors.len()));
    push(&format!("Took: {:.2?}\n", took));

    let sep = "=".repeat(max_len);
    print!("{}\n{}{}\n\n", sep, summary, sep);

    let max_bar_width_f64: f64 = config.max_bar_width as f64;
    let max_size_f64 = max_size as f64;

    for fse in results {
        let mut bar_len = if max_size == 0 {
            0
        } else {
            ((fse.size as f64 / max_size_f64) * max_bar_width_f64).round() as usize
        };

        if fse.size > 0 && bar_len == 0 {
            bar_len = 1;
        }

        let bar = "#".repeat(bar_len);
        let raw_name = &fse.name;

        let colored_name: &str = match fse.is_dir {
            Some(true) => &format!("\x1b[34m{}\x1b[0m", raw_name),
            Some(false) => &raw_name,
            None => &format!("\x1b[31m{}\x1b[0m", raw_name),
        };

        let padded_name =
            console::pad_str(colored_name, max_name_len, console::Alignment::Left, None);

        println!(
            "{name}   [{:<width_bar$}]   {:>width_size$}",
            bar,
            config.byte_unit_system.format(fse.size),
            name = padded_name,
            width_bar = config.max_bar_width as usize,
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
