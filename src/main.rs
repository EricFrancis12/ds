use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::time::Instant;

use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use regex::Regex;

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

    let match_with_regex = args.regex.is_some();
    let regex_pattern = args
        .regex
        .unwrap_or(String::from("(?s)^.*" /* matches any string */));
    let regex =
        Regex::new(&regex_pattern).expect(&format!("invalid regex pattern: {}", regex_pattern));

    let include_glob = make_globset(&args.include);
    let exclude_glob = make_globset(&args.exclude);

    let mut errors = Vec::new();

    let entries: Vec<DirEntry> = fs::read_dir(target_path)?
        .filter_map(|result| match result {
            Ok(entry) => {
                let name = entry.file_name();
                if match_with_regex {
                    match name.into_string() {
                        Ok(s) => {
                            if regex.is_match(&s) {
                                return Some(entry);
                            }
                        }
                        Err(_) => {
                            // TODO: ...
                            // errors.push(err);
                        }
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
                errors.push(err);
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

        for entry in entries {
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
            pb.inc(1);
        }

        pb.finish_and_clear();

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

fn make_globset(patterns: &Vec<String>) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for s in patterns {
        builder.add(Glob::new(&s).expect(&format!("invalid glob pattern: {}", s)));
    }
    builder.build().unwrap()
}
