mod cli;
mod config;
mod file_system;
mod filter;
mod output;
mod stats;
mod units;
mod utils;

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
use clap::{error::ErrorKind, CommandFactory};
use crossterm::{
    cursor::MoveToColumn,
    terminal::{Clear, ClearType},
};
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    cli::Args,
    config::Config,
    file_system::{entry::sort_entries, read::read_entry_recursive},
    output::{chart::print_chart, errors::print_errors, summary::print_summary},
    stats::ScanStats,
    units::system::UnitSystem,
};

fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    let config = match Config::parse(env::args()) {
        Ok(c) => Ok(c),
        Err(err) => {
            if let Some(err) = err.downcast_ref::<clap::Error>() {
                if err.kind() == ErrorKind::DisplayHelp {
                    Args::command().print_help().expect("Failed to print help");
                    return Ok(());
                } else if err.kind() == ErrorKind::DisplayVersion {
                    println!(
                        "{}",
                        Args::command()
                            .get_version()
                            .expect("Failed to print version")
                    );
                    return Ok(());
                }
            }
            Err(anyhow!("error parsing arguments into Config: {}", err))
        }
    }?;

    let target_path = Path::new(&config.dir);
    if !target_path.exists() || !target_path.is_dir() {
        return Err(anyhow!("'{}' is not a valid directory.", config.dir));
    }

    let mut results = Vec::new();
    let mut errors: Vec<anyhow::Error> = Vec::new();
    let mut stats = ScanStats::default();

    let entries: Vec<DirEntry> = fs::read_dir(target_path)?
        .filter_map(|result| match result {
            Ok(entry) => {
                if let Some(entry_type) = &config.needs_type {
                    match entry_type.try_match(&entry) {
                        Ok(true) => (/* continue on */),
                        Ok(false) => return None,
                        Err(err) => {
                            errors.push(err);
                            return None;
                        }
                    }
                }

                if let Some(filter) = &config.filter {
                    match filter.try_match(&entry) {
                        Ok(true) => (/* continue on */),
                        Ok(false) => return None,
                        Err(err) => {
                            errors.push(err);
                            return None;
                        }
                    }
                }

                Some(entry)
            }
            Err(err) => {
                errors.push(anyhow!("error reading dir entry: {}", err));
                None
            }
        })
        .collect();

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

                let fse = read_entry_recursive(
                    &entry,
                    config.unit_system == UnitSystem::Lines,
                    &mut errs,
                );

                tx.send((fse, errs)).expect("Failed to send");
            });

            handles.push(handle);
        }

        drop(tx);

        for (fse, errs) in rx {
            pb.inc(1);

            if let Some(min_size) = config.min_size {
                if fse.size < min_size {
                    continue;
                }
            }
            if let Some(max_size) = config.max_size {
                if fse.size > max_size {
                    continue;
                }
            }

            stats.apply_entry(&fse);

            results.push(fse);
            for err in errs {
                errors.push(err);
            }
        }

        for handle in handles {
            _ = handle.join();
        }

        pb.finish_and_clear();

        if let Some(sort_by) = config.sort_by {
            let mut stderr = io::stderr();

            write!(stderr, "Sorting {} results...", results.len()).unwrap();
            stderr.flush().unwrap();

            sort_entries(&mut results, &sort_by, config.reverse);

            crossterm::execute!(stderr, MoveToColumn(0), Clear(ClearType::CurrentLine)).unwrap();
        } else if config.reverse {
            results.reverse();
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
        print_errors(&errors);
    }

    print_summary(
        &config.dir,
        resolved_dir,
        &config.unit_system,
        stats.total_size,
        stats.total_lines,
        stats.dir_count,
        stats.file_count,
        stats.unknown_count,
        results.len(),
        errors.len(),
        took,
    );

    print_chart(
        &results,
        &config.unit_system,
        stats.max_size,
        stats.max_size_digits,
        stats.max_name_len,
        config.max_bar_width,
    );

    if !errors.is_empty() {
        let mut msg = format!("encountered {} error", errors.len());
        if errors.len() > 1 {
            msg.push('s');
        }
        return Err(anyhow!(msg));
    }
    Ok(())
}
