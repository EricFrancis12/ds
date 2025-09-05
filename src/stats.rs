use crate::{file_system::entry::FsEntry, utils::count_digits};

#[derive(Default)]
pub struct ScanStats {
    pub total_size: u64,
    pub total_lines: u64,
    pub max_size: u64,
    pub max_size_digits: usize,
    pub max_name_len: usize,
    pub dir_count: usize,
    pub file_count: usize,
    pub unknown_count: usize,
}

impl ScanStats {
    pub fn apply_entry(&mut self, fse: &FsEntry) {
        let name_len = fse.get_name().len();
        if name_len > self.max_name_len {
            self.max_name_len = name_len;
        }

        self.total_size += fse.size;
        if fse.size > self.max_size {
            self.max_size = fse.size;
        }

        if let Some(lines) = fse.lines {
            self.total_lines += lines;
        }

        let digits = count_digits(fse.size);
        if digits > self.max_size_digits {
            self.max_size_digits = digits;
        }

        match fse.is_dir {
            Some(true) => self.dir_count += 1,
            Some(false) => self.file_count += 1,
            None => self.unknown_count += 1,
        }
    }
}
