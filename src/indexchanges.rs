use std::path::PathBuf;

use crate::indexfile::IndexFile;

#[derive(Debug)]
pub enum IndexChange {
    /// Ensure a directory with this path exists (at least if all its parent directories exist).
    AddDir(PathBuf, bool, u64),
    /// Add or update a file
    AddFile(PathBuf, IndexFile),
    /// Same as `AddFile`, just that it creates a symlink pointing to the 2nd path
    AddSymlink(PathBuf, PathBuf),
    /// Remove a file or symlink
    RemoveFile(PathBuf),
    /// Remove a directory (recursively)
    RemoveDir(PathBuf),
}
