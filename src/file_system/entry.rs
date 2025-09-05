pub struct FsEntry {
    pub name: Option<String>,
    pub is_dir: Option<bool>,
    pub size: u64,
    pub lines: Option<u64>,
}
