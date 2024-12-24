use std::path::PathBuf;

use crate::indexfile::IndexFile;

#[derive(Debug)]
pub enum IndexChange {
    /// Ensure a directory with this path exists (at least if all its parent directories exist).
    AddDir(PathBuf, u64),
    /// Add or update a file
    AddFile(PathBuf, IndexFile),
    /// Remove a file
    RemoveFile(PathBuf),
    /// Remove a directory (recursively)
    RemoveDir(PathBuf),
}
