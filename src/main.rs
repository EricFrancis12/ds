use std::fs::{self, DirEntry};
use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use anyhow::anyhow;
use clap::Parser;
use crossterm::{
    cursor::MoveToColumn,
    terminal::{Clear, ClearType},
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

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
    #[arg(name = "regex", long = "regex", short = 'r')]
    regex: Option<String>,
    #[arg(
        name = "include",
        long = "include",
        short = 'i',
        conflicts_with = "regex"
    )]
    include: Vec<String>,
    #[arg(
        name = "exclude",
        long = "exclude",
        short = 'e',
        conflicts_with = "regex"
    )]
    exclude: Vec<String>,
    #[arg(
        name = "max bar width",
        long = "bw",
        aliases = vec!["bl", "bs"],
        default_value = "50"
    )]
    max_bar_width: u32,
    #[arg(
        name = "no errors",
        long = "no-errors",
        aliases = vec![
            "no-error",
            "no-errs",
            "no-err",
            "noerrors",
            "noerror",
            "noerrs",
            "noerr"
        ]
    )]
    no_errors: bool,
}

struct FsEntry {
    name: String,
    size: u64,
    errors: Vec<anyhow::Error>,
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

        let (tx, rx) = mpsc::channel::<FsEntry>();
        let mut handles = Vec::new();

        const UNKNOWN_ENTRY: &str = "[Unknown Entry]";

        for entry in entries {
            let tx = tx.clone();
            let handle = thread::spawn(move || {
                let mut errors = Vec::new();
                let name = entry.file_name().into_string().unwrap_or_else(|_| {
                    errors.push(anyhow!(
                        "error getting entry name (entry will be named {} in results)",
                        UNKNOWN_ENTRY
                    ));
                    String::from(UNKNOWN_ENTRY)
                });

                let size = get_size(&entry.path()).unwrap_or_else(|err| {
                    errors.push(anyhow!("error getting size for '{}': {}", name, err));
                    0
                });

                let fse = FsEntry { name, size, errors };
                tx.send(fse).expect("Failed to send");
            });

            handles.push(handle);
        }

        drop(tx);

        for fse in rx {
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

            results.push((fse.name, fse.size));
            for err in fse.errors {
                errors.push(err);
            }

            pb.inc(1);
        }

        for handle in handles {
            let _ = handle.join();
        }

        pb.finish_and_clear();

        if args.sort_by_name || args.sort_by_size {
            let mut stderr = io::stderr();

            write!(stderr, "Sorting {} results...", results.len()).unwrap();
            stderr.flush().unwrap();

            if args.sort_by_name {
                results.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
            } else if args.sort_by_size {
                results.sort_by(|a, b| b.1.cmp(&a.1));
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
        fmt_bytes(total_size, args.bytes_readable)
    ));
    push(&format!("Items: {}\n", results.len()));
    push(&format!("Errors: {}\n", errors.len()));
    push(&format!("Took: {:.2?}\n", took));

    let sep = "=".repeat(max_len);
    print!("{}\n{}{}\n\n", sep, summary, sep);

    let max_bar_width_f64: f64 = args.max_bar_width as f64;
    let max_size_f64 = max_size as f64;

    for (name, size) in results {
        let mut bar_len = if max_size == 0 {
            0
        } else {
            ((size as f64 / max_size_f64) * max_bar_width_f64).round() as usize
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

fn make_globset(patterns: &Vec<String>) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for s in patterns {
        builder.add(Glob::new(&s).expect(&format!("invalid glob pattern: {}", s)));
    }
    builder.build().unwrap()
}
