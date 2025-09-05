use std::{
    fs::{self, DirEntry, File},
    io::{BufReader, Read},
};

use anyhow::anyhow;

use crate::{file_system::entry::FsEntry, ok_or};

pub fn read_entry_recursive(
    entry: &DirEntry,
    count_lines: bool,
    errors: &mut Vec<anyhow::Error>,
) -> FsEntry {
    let name = match entry.file_name().into_string() {
        Ok(s) => Some(s),
        Err(_) => {
            errors.push(anyhow!(
                "error getting entry name (entry will be named {} in results)",
                FsEntry::UNKNOWN_ENTRY
            ));
            None
        }
    };

    let is_dir = match entry.metadata() {
        Ok(m) => Some(m.is_dir()),
        Err(err) => {
            let name = name.as_deref().unwrap_or(FsEntry::UNKNOWN_ENTRY);
            errors.push(anyhow!("error getting metadata for '{}': {}", name, err));
            None
        }
    };

    let (size, lines) = read_entry_recursive_internal(&entry, count_lines, errors);

    FsEntry {
        name,
        is_dir,
        size,
        lines,
    }
}

fn read_entry_recursive_internal(
    entry: &DirEntry,
    count_lines: bool,
    errors: &mut Vec<anyhow::Error>,
) -> (u64, Option<u64>) {
    let path = entry.path();

    if path.is_file() {
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        let lines = if count_lines {
            match read_and_count_lines(entry) {
                Ok(lines) => Some(lines),
                Err(err) => {
                    errors.push(err);
                    None
                }
            }
        } else {
            None
        };

        (size, lines)
    } else if path.is_dir() {
        let it = ok_or!(fs::read_dir(&path), err => {
            errors.push(anyhow!(
                "error reading dir '{}': {err}",
                path.to_string_lossy()
            ));
            return (0, None);
        });

        let mut size = 0;
        let mut lines = if count_lines { Some(0) } else { None };

        for en in it {
            let en = ok_or!(en, err => {
                errors.push(anyhow!(
                    "dir entry read error for '{}': {err}",
                    path.to_string_lossy()
                ));
                continue;
            });

            let (s, l) = read_entry_recursive_internal(&en, count_lines, errors);
            size += s;
            if let Some(l) = l {
                lines = lines.map(|n| n + l);
            }
        }

        (size, lines)
    } else {
        errors.push(anyhow!(
            "could not determine if '{}' is a file or directory",
            path.to_string_lossy()
        ));
        (0, None)
    }
}

fn read_and_count_lines(entry: &DirEntry) -> anyhow::Result<u64> {
    let file = File::open(entry.path())?;

    let mut reader = BufReader::new(file);
    const CHUNK_SIZE: usize = 100;
    let mut buffer = vec![0u8; CHUNK_SIZE];

    let mut lines = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let chunk = &buffer[..bytes_read];

        if chunk.contains(&0) {
            // likely a binary file, so skip entire file
            return Ok(0);
        }

        match str::from_utf8(chunk) {
            Ok(s) => lines += s.lines().count() as u64,
            // likely a binary file, so skip entire file
            Err(_) => return Ok(0),
        }
    }

    Ok(lines)
}
