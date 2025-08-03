use std::fs::{self, DirEntry};
use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use anyhow::anyhow;
use clap::{builder::PossibleValue, Parser, ValueEnum};
use crossterm::{
    cursor::MoveToColumn,
    terminal::{Clear, ClearType},
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

#[derive(Clone, Debug)]
pub enum ByteUnitSystem {
    Raw,
    SI,
    Binary,
}

impl Default for ByteUnitSystem {
    fn default() -> Self {
        Self::Raw
    }
}

impl ValueEnum for ByteUnitSystem {
    fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
        let input = if ignore_case {
            input.to_lowercase()
        } else {
            input.to_owned()
        };
        match input.as_str() {
            "" | "raw" => Ok(Self::Raw),
            "si" | "1000" => Ok(Self::SI),
            "binary" | "bin" | "1024" => Ok(Self::Binary),
            s => Err(s.to_owned()),
        }
    }

    fn value_variants<'a>() -> &'a [Self] {
        static VARIANTS: [ByteUnitSystem; 3] = [
            ByteUnitSystem::Raw,
            ByteUnitSystem::SI,
            ByteUnitSystem::Binary,
        ];
        &VARIANTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            ByteUnitSystem::Raw => PossibleValue::new("raw").help("Raw bytes with no scaling"),
            ByteUnitSystem::SI => {
                // TODO: define aliases for use here and in from_str
                PossibleValue::new("si").aliases(["1000"]).help(format!(
                    "SI units (base 1000): {}",
                    Self::SI_UNITS.join(", ")
                ), /* TODO: format at compile time */)
            }
            ByteUnitSystem::Binary => {
                // TODO: define aliases for use here and in from_str
                PossibleValue::new("binary").aliases(["bin", "1024"]).help(
                    format!(
                        "Binary units (base 1024): {}",
                        Self::BINARY_UNITS.join(", ")
                    ), /* TODO: format at compile time */
                )
            }
        })
    }
}

impl ByteUnitSystem {
    const SI_UNITS: [&str; 7] = ["B", "kB", "MB", "GB", "TB", "PB", "EB"];
    const BINARY_UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

    fn format(&self, bytes: u64) -> String {
        match self {
            ByteUnitSystem::Raw => format!("{}", bytes),
            ByteUnitSystem::SI => Self::do_format(bytes, 1000, Self::SI_UNITS),
            ByteUnitSystem::Binary => Self::do_format(bytes, 1024, Self::BINARY_UNITS),
        }
    }

    fn do_format(bytes: u64, base: u32, units: [&str; 7]) -> String {
        let mut value = bytes as f64;
        let base = base as f64;
        let mut unit = units[0];

        for &next_unit in &units[1..] {
            if value < base {
                break;
            }
            value /= base;
            unit = next_unit;
        }

        format!("{:.2} {}", value, unit)
    }
}

struct FsEntry {
    name: String,
    size: u64,
    is_dir: Option<bool>,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(default_value = ".")]
    dir: String,
    #[arg(name = "name", long = "name", short = 'n', conflicts_with_all = vec!["size", "type"])]
    sort_by_name: bool,
    #[arg(name = "size", long = "size", short = 's', conflicts_with_all = vec!["name", "type"])]
    sort_by_size: bool,
    #[arg(name = "type", long = "type", short = 't', conflicts_with_all = vec!["name", "size"])]
    sort_by_type: bool,
    #[arg(
        name = "byte-unit-system",
        long = "byte-unit-system",
        short = 'b',
        aliases = vec!["bytes", "bus"],
        value_enum,
        default_value_t = ByteUnitSystem::default()
    )]
    byte_unit_system: ByteUnitSystem,
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
        name = "max-bar-width",
        long = "max-bar-width",
        aliases = vec!["bw", "bl", "bs"],
        default_value = "50"
    )]
    max_bar_width: u32,
    #[arg(
        name = "no-errors",
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
