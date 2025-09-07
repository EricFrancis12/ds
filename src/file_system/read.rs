use std::{
    fs::{self, DirEntry, File},
    io::{BufReader, Read},
    os::windows::fs::MetadataExt,
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
};

use anyhow::anyhow;

use crate::{file_system::entry::FsEntry, ok_or, utils::sync::Semaphore};

pub fn spawn_readers(
    entries: Vec<DirEntry>, // TODO: refactor to be a &[DirEntry] ?
    count_lines: bool,
) -> (Receiver<(FsEntry, Vec<anyhow::Error>)>, Vec<JoinHandle<()>>) {
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();

    // TODO: add max_threads to Config
    let sem = Arc::new(Semaphore::new(20));

    for entry in entries {
        let sem = sem.clone();
        sem.lock();
        let tx = tx.clone();

        let handle = thread::spawn(move || {
            let mut errs = Vec::new();

            let fse = read_entry_recursive(&entry, count_lines, &mut errs);

            tx.send((fse, errs)).expect(&format!(
                "Reader thread '{}' failed to send",
                entry.path().to_string_lossy()
            ));
            sem.unlock();
        });

        handles.push(handle);
    }

    (rx, handles)
}

fn read_entry_recursive(
    entry: &DirEntry,
    count_lines: bool,
    errors: &mut Vec<anyhow::Error>,
) -> FsEntry {
    let name = entry.file_name();

    let metadata = ok_or!(entry.metadata(), err =>  {
        errors.push(anyhow!(
            "error getting metadata for '{}': {}",
            name.to_string_lossy(),
            err
        ));
        return FsEntry::Unknown { name, size: None };
    });

    if metadata.is_file() {
        let lines = match count_lines {
            true => match read_and_count_lines(entry) {
                Ok(lines) => Some(lines),
                Err(err) => {
                    errors.push(err);
                    None
                }
            },
            false => None,
        };

        return FsEntry::File {
            name,
            size: metadata.file_size(),
            lines,
        };
    }

    if metadata.is_dir() {
        let path = entry.path();
        let children = match fs::read_dir(&path) {
            Ok(it) => {
                let mut children = Vec::new();
                for result in it {
                    let en = ok_or!(result , err => {
                        errors.push(anyhow!(
                            "error reading dir entry '{}': {}",
                            entry.file_name().to_string_lossy(),
                            err
                        ));
                        continue;
                    });
                    children.push(
                        // TODO: this should be done in a new thread
                        read_entry_recursive(&en, count_lines, errors),
                    );
                }
                Some(children)
            }
            Err(err) => {
                errors.push(anyhow!(
                    "error reading dir '{}': {err}",
                    path.to_string_lossy()
                ));
                None
            }
        };

        return FsEntry::Dir {
            name,
            size: metadata.file_size(),
            children,
        };
    }

    FsEntry::Unknown { name, size: None }
}

fn read_and_count_lines(entry: &DirEntry) -> anyhow::Result<u64> {
    let file = File::open(entry.path())?;
    let mut reader = BufReader::new(file);

    // A low byte chunk size should be used here because we want to quickly disqualify files
    // that are not valid UTF-8. Smaller chunks allow us to detect non-UTF8 files earlier,
    // making the check faster in practice for binary files.
    const CHUNK_SIZE: usize = 128;

    let mut buffer = [0u8; CHUNK_SIZE];
    let mut leftover = Vec::new(); // TODO: use OnceCell for leftover?
    let mut lines = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        // Combine leftover bytes from previous chunk
        let mut chunk = leftover.clone();
        chunk.extend_from_slice(&buffer[..bytes_read]);

        match str::from_utf8(&chunk) {
            Ok(s) => {
                lines += s.lines().count() as u64;
                leftover.clear();
            }
            Err(err) => {
                // Save incomplete bytes for next chunk
                let valid_up_to = err.valid_up_to();
                if valid_up_to == 0 {
                    // No valid UTF-8, likely a binary file
                    return Ok(0);
                }
                let valid = &chunk[..valid_up_to];
                lines += str::from_utf8(valid).unwrap().lines().count() as u64;
                leftover = chunk[valid_up_to..].to_vec();
            }
        }
    }

    // If leftover bytes remain, check if they form a valid UTF-8 character
    if !leftover.is_empty() {
        if let Ok(s) = str::from_utf8(&leftover) {
            lines += s.lines().count() as u64;
        } else {
            // invalid UTF-8, likely a binary file
            return Ok(0);
        }
    }

    Ok(lines)
}
