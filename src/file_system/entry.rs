pub const UNKNOWN_ENTRY: &str = "[Unknown Entry]";
pub const UNKNOWN_ENTRY_LEN: usize = UNKNOWN_ENTRY.len();

pub struct FsEntry {
    pub name: Option<String>,
    pub size: u64,
    pub is_dir: Option<bool>,
}
