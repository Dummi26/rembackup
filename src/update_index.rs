use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use clap::Args;

use crate::{
    config::{FsEntry, Ignore, Match, Specifier},
    indexchanges::IndexChange,
    indexfile::IndexFile,
};

#[derive(Clone, Default, Args)]
pub struct Settings {
    /// don't sort the changes that form the backup
    ///
    /// disables sort options.
    /// symlinks will still be created last.
    #[arg(long)]
    pub dont_sort: bool,
    /// start with smaller directories rather than larger ones
    ///
    /// symlinks will still be created last.
    #[arg(long)]
    pub smallest_first: bool,
    /// show changes in the order in which they will be applied, not reversed
    #[arg(long)]
    pub dont_reverse_output: bool,

    /// don't update files just because their timestamp is different
    #[arg(long)]
    pub ignore_timestamp: bool,
    /// keep *newer* files when the files in the source are *older*
    #[arg(long)]
    pub dont_replace_newer: bool,
    /// replace files if their timestamp is unknown in both source and index
    #[arg(long)]
    pub replace_if_timestamp_unknown: bool,
    /// replace files if their timestamp is unknown in source but known in index
    #[arg(long)]
    pub replace_if_timestamp_lost: bool,
    /// don't replace files if their timestamp is known in source but unknown in index
    #[arg(long)]
    pub dont_replace_if_timestamp_found: bool,
}

pub fn perform_index_diff<'a>(
    source: &Path,
    index: &'a Path,
    target: Option<&'a Path>,
    mut ignore: Ignore,
    settings: &Settings,
    sort_by_size_largest: Option<bool>,
) -> Result<(u64, Vec<IndexChange>), (String, PathBuf, io::Error)> {
    if let Ok(inner_index) = index.strip_prefix(source) {
        eprintln!("[info] source contains index at {inner_index:?}, but index will not be part of the backup.");
        ignore.0.push(Specifier::InDir {
            dir: Match::Eq(inner_index.to_owned()),
            inner: Ignore(vec![]),
        });
    }
    if let Some(target) = target {
        if let Ok(inner_target) = target.strip_prefix(source) {
            eprintln!("[info] source contains target at {inner_target:?}, but target will not be part of the backup.");
            ignore.0.push(Specifier::InDir {
                dir: Match::Eq(inner_target.to_owned()),
                inner: Ignore(vec![]),
            });
        }
    }
    if let Some((total_size, changes)) = rec(
        source.as_ref(),
        Path::new(""),
        index,
        &ignore,
        settings,
        sort_by_size_largest,
    )? {
        Ok((total_size, changes))
    } else {
        Ok((0, vec![]))
    }
}
fn rec(
    // location of source files
    source: &Path,
    // relative path used on this iteration
    rel_path: &Path,
    // location of the index
    index_files: &Path,
    ignore: &Ignore,
    settings: &Settings,
    sort_by_size_largest: Option<bool>,
) -> Result<Option<(u64, Vec<IndexChange>)>, (String, PathBuf, io::Error)> {
    let mut removals = vec![];
    let mut ichanges = vec![];
    let mut total_size = 0;
    // used to find removals
    let index_rel_path = index_files.join(rel_path);
    let (mut index_entries, dir_is_new) = match fs::read_dir(&index_rel_path) {
        Err(_) => (HashMap::new(), true),
        Ok(e) => (
            e.into_iter()
                .filter_map(|v| v.ok())
                .map(|v| {
                    Ok((
                        v.file_name(),
                        v.file_type()
                            .map_err(|e| ("getting file type".to_owned(), v.path(), e))?
                            .is_dir(),
                    ))
                })
                .collect::<Result<_, (String, PathBuf, io::Error)>>()?,
            false,
        ),
    };
    // compare source files with index
    let source_files_path = source.join(rel_path);
    let source_files = fs::read_dir(&source_files_path)
        .map_err(|e| ("getting entries".to_owned(), source_files_path.clone(), e))?
        .collect::<Vec<_>>();
    // find changes/adds
    for entry in source_files {
        let entry = entry.map_err(|e| {
            (
                "error with an entry within this directory".to_owned(),
                source_files_path.clone(),
                e,
            )
        })?;
        let entry_path = entry.path();
        let rel_path = rel_path.join(entry.file_name());
        let metadata = fs::symlink_metadata(&entry_path);

        // ignore entries
        let fs_entry = FsEntry {
            path: &rel_path,
            is_directory: metadata.as_ref().ok().map(|v| v.is_dir()),
        };
        if ignore.matches_or_default(&fs_entry) {
            continue;
        }

        let metadata = metadata.map_err(|e| ("getting metadata (you have to ignore this using a * pattern instead of + or /, because we don't know if it's a directory or not)".to_owned(), entry_path.clone(), e))?;
        let in_index_and_is_dir = index_entries.remove(&entry.file_name());
        if metadata.is_dir() {
            if let Some(false) = in_index_and_is_dir {
                // is dir, but was file -> remove file
                removals.push(IndexChange::RemoveFile(rel_path.clone()));
            }
            if let Some((rec_size, rec_changes)) = rec(
                source,
                &rel_path,
                index_files,
                ignore,
                settings,
                sort_by_size_largest,
            )? {
                total_size += rec_size;
                ichanges.push((rec_size, rec_changes));
            }
        } else {
            if let Some(true) = in_index_and_is_dir {
                // is file, but was dir -> remove dir
                removals.push(IndexChange::RemoveDir(rel_path.clone()));
            }
            let index_file_path = index_files.join(&rel_path);
            let new_is_symlink = metadata.is_symlink();
            let old_is_symlink = index_file_path
                .symlink_metadata()
                .is_ok_and(|meta| meta.is_symlink());
            if new_is_symlink && old_is_symlink {
                // cd to file's parent directory, in case of relative links, just to be sure
                let cwd = std::env::current_dir()
                    .map_err(|e| (format!("couldn't get CWD"), entry_path.clone(), e))?;
                std::env::set_current_dir(&source_files_path).map_err(|e| {
                    (
                        format!("could not set CWD to {}", source_files_path.display()),
                        entry_path.clone(),
                        e,
                    )
                })?;
                let new_link = fs::read_link(&entry_path).map_err(|e| {
                    (
                        format!("couldn't read symlink contents"),
                        entry_path.clone(),
                        e,
                    )
                })?;
                std::env::set_current_dir(&index_rel_path).map_err(|e| {
                    (
                        format!("could not set CWD to {}", index_rel_path.display()),
                        entry_path.clone(),
                        e,
                    )
                })?;
                let old_link = fs::read_link(&index_file_path).map_err(|e| {
                    (
                        format!("couldn't read indexfile symlink contents"),
                        entry_path.clone(),
                        e,
                    )
                })?;
                std::env::set_current_dir(&cwd).map_err(|e| {
                    (
                        format!("could not reset CWD to {}", cwd.display()),
                        entry_path.clone(),
                        e,
                    )
                })?;
                if new_link != old_link {
                    ichanges.push((0, vec![IndexChange::AddSymlink(rel_path, new_link)]));
                }
            } else if new_is_symlink {
                let cwd = std::env::current_dir()
                    .map_err(|e| (format!("couldn't get CWD"), entry_path.clone(), e))?;
                std::env::set_current_dir(&source_files_path).map_err(|e| {
                    (
                        format!("could not set CWD to {}", source_files_path.display()),
                        entry_path.clone(),
                        e,
                    )
                })?;
                let new_link = fs::read_link(&entry_path).map_err(|e| {
                    (
                        format!("couldn't read symlink contents"),
                        entry_path.clone(),
                        e,
                    )
                })?;
                std::env::set_current_dir(&cwd).map_err(|e| {
                    (
                        format!("could not reset CWD to {}", cwd.display()),
                        entry_path.clone(),
                        e,
                    )
                })?;
                if let Some(false) = in_index_and_is_dir {
                    // was file before
                    removals.push(IndexChange::RemoveFile(rel_path.clone()));
                }
                ichanges.push((0, vec![IndexChange::AddSymlink(rel_path, new_link)]));
            } else {
                let newif = IndexFile::new_from_metadata(&metadata);
                let oldif = IndexFile::from_path(&index_file_path);
                if old_is_symlink {
                    removals.push(IndexChange::RemoveFile(rel_path.clone()));
                }
                match oldif {
                    Ok(Ok(oldif)) if !newif.should_be_updated(&oldif, settings) => {}
                    _ => {
                        total_size += newif.size;
                        ichanges.push((newif.size, vec![IndexChange::AddFile(rel_path, newif)]));
                    }
                }
            }
        }
    }
    // removals
    for (removed_file, is_dir) in index_entries {
        removals.push(if is_dir {
            IndexChange::RemoveDir(rel_path.join(removed_file))
        } else {
            IndexChange::RemoveFile(rel_path.join(removed_file))
        });
    }
    // sorting
    if let Some(sort_largest_first) = sort_by_size_largest {
        if sort_largest_first {
            ichanges.sort_by(|a, b| b.0.cmp(&a.0));
        } else {
            ichanges.sort_by_key(|v| v.0);
        }
    }
    // combine everything
    if !dir_is_new && removals.is_empty() && ichanges.is_empty() {
        return Ok(None);
    }
    let changes = [IndexChange::AddDir(
        rel_path.to_path_buf(),
        dir_is_new,
        total_size,
    )]
    .into_iter()
    .chain(removals.into_iter())
    .chain(ichanges.into_iter().flat_map(|(_, v)| v))
    .collect();
    Ok(Some((total_size, changes)))
}
