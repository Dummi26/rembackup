use std::{collections::HashMap, fs, io, path::Path};

use clap::Args;

use crate::{
    config::{FsEntry, Ignore, Match, Specifier},
    indexchanges::IndexChange,
    indexfile::IndexFile,
};

#[derive(Clone, Default, Args)]
pub struct Settings {
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
) -> io::Result<Vec<IndexChange>> {
    let mut changes = Vec::new();
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
    rec(
        source.as_ref(),
        Path::new(""),
        index,
        &mut changes,
        &ignore,
        settings,
    )?;
    Ok(changes)
}
fn rec(
    // location of source files
    source: &Path,
    // relative path used on this iteration
    rel_path: &Path,
    // location of the index
    index_files: &Path,
    // list of changes to be made
    changes: &mut Vec<IndexChange>,
    ignore: &Ignore,
    settings: &Settings,
) -> Result<(), io::Error> {
    // used to find removals
    let index_rel_path = index_files.join(rel_path);
    let mut index_entries = match fs::read_dir(&index_rel_path) {
        Err(_) => {
            changes.push(IndexChange::AddDir(rel_path.to_path_buf()));
            HashMap::new()
        }
        Ok(e) => e
            .into_iter()
            .filter_map(|v| v.ok())
            .map(|v| Ok((v.file_name(), v.file_type()?.is_dir())))
            .collect::<Result<_, io::Error>>()?,
    };
    // compare source files with index
    let source_files = fs::read_dir(source.join(rel_path))?.collect::<Vec<_>>();
    // find changes/adds
    for entry in source_files {
        let entry = entry?;
        let rel_path = rel_path.join(entry.file_name());
        let metadata = entry.metadata()?;

        // ignore entries
        let fs_entry = FsEntry {
            path: &rel_path,
            is_directory: metadata.is_dir(),
        };
        if ignore.matches_or_default(&fs_entry) {
            continue;
        }

        let in_index_and_is_dir = index_entries.remove(&entry.file_name());
        if metadata.is_dir() {
            if let Some(false) = in_index_and_is_dir {
                // is dir, but was file -> remove file
                changes.push(IndexChange::RemoveFile(rel_path.clone()));
            }
            rec(source, &rel_path, index_files, changes, ignore, settings)?;
        } else {
            if let Some(true) = in_index_and_is_dir {
                // is file, but was dir -> remove dir
                changes.push(IndexChange::RemoveDir(rel_path.clone()));
            }
            let newif = IndexFile::new_from_metadata(&metadata);
            let oldif = IndexFile::from_path(&index_files.join(&rel_path));
            match oldif {
                Ok(Ok(oldif)) if !newif.should_be_updated(&oldif, settings) => {}
                _ => changes.push(IndexChange::AddFile(rel_path, newif)),
            }
        }
    }
    // removals
    for (removed_file, is_dir) in index_entries {
        changes.push(if is_dir {
            IndexChange::RemoveDir(rel_path.join(removed_file))
        } else {
            IndexChange::RemoveFile(rel_path.join(removed_file))
        });
    }
    Ok(())
}
