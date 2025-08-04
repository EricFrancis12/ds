pub struct FsEntry {
    pub name: String, // TODO: should this be Option<String>, because could fail to get name from OsString
    pub size: u64,
    pub is_dir: Option<bool>,
}
