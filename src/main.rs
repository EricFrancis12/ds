mod bytes;
mod cli;
mod config;
mod entry;
mod filter;
mod output;

use std::{
    cmp::Ordering,
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
    entry::{FsEntry, UNKNOWN_ENTRY, UNKNOWN_ENTRY_LEN},
    output::{chart::print_chart, summary::print_summary},
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

    let mut results = Vec::new();
    let mut total_size = 0;
    let mut max_size = 0;
    let mut max_size_digits = 0;
    let mut max_name_len = 0;

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

        for entry in entries {
            let tx = tx.clone();
            let handle = thread::spawn(move || {
                let mut errs = Vec::new();

                let name = match entry.file_name().into_string() {
                    Ok(s) => Some(s),
                    Err(_) => {
                        errs.push(anyhow!(
                            "error getting entry name (entry will be named {} in results)",
                            UNKNOWN_ENTRY
                        ));
                        None
                    }
                };

                let size = get_size(&entry.path()).unwrap_or_else(|err| {
                    let name = name.as_deref().unwrap_or(UNKNOWN_ENTRY);
                    errs.push(anyhow!("error getting size for '{}': {}", name, err));
                    0
                });

                let is_dir = match entry.metadata() {
                    Ok(m) => Some(m.is_dir()),
                    Err(err) => {
                        let name = name.as_deref().unwrap_or(UNKNOWN_ENTRY);
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
            let name_len = match fse.name.as_ref() {
                Some(name) => name.len(),
                None => UNKNOWN_ENTRY_LEN,
            };
            if name_len > max_name_len {
                max_name_len = name_len;
            }

            total_size += fse.size;
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
                    results.sort_by(|a: &FsEntry, b| match (&a.name, &b.name) {
                        (Some(a_name), Some(b_name)) => {
                            a_name.to_lowercase().cmp(&b_name.to_lowercase())
                        }
                        (Some(_), None) => Ordering::Greater,
                        (None, Some(_)) => Ordering::Less,
                        (None, None) => Ordering::Equal,
                    });
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

    print_summary(
        &config.dir,
        resolved_dir,
        &config.byte_unit_system,
        total_size,
        results.len(),
        errors.len(),
        took,
    );

    print_chart(
        &results,
        &config.byte_unit_system,
        max_size,
        max_size_digits,
        max_name_len,
        config.max_bar_width,
    );

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
