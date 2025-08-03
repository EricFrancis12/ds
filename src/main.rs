mod bytes;
mod cli;

use std::{
    fs::{self, DirEntry},
    io::{self, Write},
    path::Path,
    sync::mpsc,
    thread,
    time::Instant,
};

use anyhow::anyhow;
use clap::Parser;
use crossterm::{
    cursor::MoveToColumn,
    terminal::{Clear, ClearType},
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

use crate::cli::Args;

struct FsEntry {
    name: String, // TODO: should this be Option<String>, because could fail to get name from OsString
    size: u64,
    is_dir: Option<bool>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let start = Instant::now();
    let target_path = Path::new(&args.dir);

    if !target_path.exists() || !target_path.is_dir() {
        return Err(anyhow!("'{}' is not a valid directory.", args.dir));
    }

    let match_with_regex = args.regex.is_some();
    let regex_pattern = args
        .regex
        .unwrap_or(String::from("(?s)^.*" /* matches any string */));
    let regex =
        Regex::new(&regex_pattern).expect(&format!("invalid regex pattern: {}", regex_pattern));

    let include_glob = make_globset(&args.include);
    let exclude_glob = make_globset(&args.exclude);

    let mut errors: Vec<anyhow::Error> = Vec::new();

    let entries: Vec<DirEntry> = fs::read_dir(target_path)?
        .filter_map(|result| match result {
            Ok(entry) => {
                let name = entry.file_name();
                if match_with_regex {
                    match name.to_str() {
                        Some(s) => {
                            if regex.is_match(s) {
                                return Some(entry);
                            }
                        }
                        None => errors.push(anyhow!(
                            "cannot convert OsString to &str; skipping regex match"
                        )),
                    }
                } else {
                    if (include_glob.is_empty() || include_glob.is_match(&name))
                        && (exclude_glob.is_empty() || !exclude_glob.is_match(name))
                    {
                        return Some(entry);
                    }
                }
                None
            }
            Err(err) => {
                errors.push(anyhow!("error reading item entry: {}", err));
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

        if args.sort_by_name || args.sort_by_size || args.sort_by_type {
            let mut stderr = io::stderr();

            write!(stderr, "Sorting {} results...", results.len()).unwrap();
            stderr.flush().unwrap();

            if args.sort_by_name {
                results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            } else if args.sort_by_size {
                results.sort_by(|a, b| b.size.cmp(&a.size));
            } else if args.sort_by_type {
                results.sort_by(|a, b| {
                    let cmp_val = |is_dir: Option<bool>| match is_dir {
                        Some(true) => 0,
                        Some(false) => 1,
                        None => 2,
                    };
                    cmp_val(a.is_dir).cmp(&cmp_val(b.is_dir))
                });
            }

            crossterm::execute!(stderr, MoveToColumn(0), Clear(ClearType::CurrentLine)).unwrap();
        }
    }

    let resolved_dir: &str = match fs::canonicalize(Path::new(&args.dir)) {
        Ok(path) => &format!("{}", path.to_str().unwrap_or(&args.dir)),
        Err(err) => {
            errors.push(anyhow!(
                "error resolving full path for '{}': {}",
                args.dir,
                err
            ));
            &args.dir
        }
    };

    let took = start.elapsed();

    if !args.no_errors && !errors.is_empty() {
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

    push(&format!("File/Directory Sizes in '{}'\n", args.dir));
    push(&format!("Resolved Path: {}\n", resolved_dir));
    push(&format!(
        "Total Size: {}\n",
        args.byte_unit_system.format(total_size)
    ));
    push(&format!("Items: {}\n", results.len()));
    push(&format!("Errors: {}\n", errors.len()));
    push(&format!("Took: {:.2?}\n", took));

    let sep = "=".repeat(max_len);
    print!("{}\n{}{}\n\n", sep, summary, sep);

    let max_bar_width_f64: f64 = args.max_bar_width as f64;
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
            args.byte_unit_system.format(fse.size),
            name = padded_name,
            width_bar = args.max_bar_width as usize,
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

fn make_globset(patterns: &Vec<String>) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for s in patterns {
        builder.add(Glob::new(&s).expect(&format!("invalid glob pattern: {}", s)));
    }
    builder.build().unwrap()
}
