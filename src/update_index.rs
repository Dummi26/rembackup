use std::{collections::HashMap, fs, io, path::Path};

use crate::{indexchanges::IndexChange, indexfile::IndexFile};

pub fn perform_index_diff<'a>(
    source: &Path,
    index: &'a Path,
    mut ignore_paths: Vec<&'a Path>,
) -> io::Result<Vec<IndexChange>> {
    let mut changes = Vec::new();
    if let Ok(inner_index) = index.strip_prefix(source) {
        eprintln!("[info] source contains index, but index will not be part of the backup.");
        ignore_paths.push(inner_index);
    }
    rec(
        source.as_ref(),
        Path::new(""),
        index,
        &mut changes,
        &ignore_paths,
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
    ignore_paths: &Vec<&Path>,
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
        // ignore entries
        if ignore_paths.iter().any(|ii| &rel_path == ii) {
            continue;
        }

        let metadata = entry.metadata()?;
        let in_index_and_is_dir = index_entries.remove(&entry.file_name());
        if metadata.is_dir() {
            if let Some(false) = in_index_and_is_dir {
                // is dir, but was file -> remove file
                changes.push(IndexChange::RemoveFile(rel_path.clone()));
            }
            rec(source, &rel_path, index_files, changes, ignore_paths)?;
        } else {
            if let Some(true) = in_index_and_is_dir {
                // is file, but was dir -> remove dir
                changes.push(IndexChange::RemoveDir(rel_path.clone()));
            }
            let newif = IndexFile::new_from_metadata(&metadata);
            let oldif = IndexFile::from_path(&index_files.join(&rel_path));
            match oldif {
                Ok(Ok(oldif)) if oldif == newif => {}
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
