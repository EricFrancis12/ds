use crate::{file_system::entry::FsEntry, utils::math::count_digits};

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
        let name_len = fse.name_str().len();
        if name_len > self.max_name_len {
            self.max_name_len = name_len;
        }

        if let Some(size) = fse.size() {
            self.total_size += size;
            if size > self.max_size {
                self.max_size = size;
            }

            let digits = count_digits(size);
            if digits > self.max_size_digits {
                self.max_size_digits = digits;
            }
        }

        if let FsEntry::File { lines, .. } = fse {
            if let Some(lines) = lines {
                self.total_lines += lines;
            }
        }

        match fse {
            FsEntry::File { .. } => self.file_count += 1,
            FsEntry::Dir { .. } => self.dir_count += 1,
            FsEntry::Unknown { .. } => self.unknown_count += 1,
        }
    }
}
