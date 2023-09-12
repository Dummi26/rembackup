use std::{fs, io, path::Path};

use crate::{indexchanges::IndexChange, indexfile::IndexFile};

pub fn perform_index_diff(source: &Path, index: &Path) -> io::Result<Vec<IndexChange>> {
    let mut changes = Vec::new();
    rec(
        source.as_ref(),
        Path::new(""),
        index,
        &mut changes,
        index.strip_prefix(source).ok(),
    )?;
    Ok(changes)
}
fn rec(
    source: &Path,
    rel_path: &Path,
    index_files: &Path,
    changes: &mut Vec<IndexChange>,
    inner_index: Option<&Path>,
) -> Result<(), io::Error> {
    if let Some(ii) = &inner_index {
        if rel_path.starts_with(ii) {
            eprintln!("[info] source contains index, but index will not be part of the backup.");
            return Ok(());
        }
    }

    if !index_files.join(rel_path).try_exists()? {
        changes.push(IndexChange::AddDir(rel_path.to_path_buf()));
    }
    for entry in fs::read_dir(source.join(rel_path))? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            rec(
                source,
                &rel_path.join(entry.file_name()),
                index_files,
                changes,
                inner_index,
            )?;
        } else {
            let newif = IndexFile::new_from_metadata(&metadata);
            let oldif = IndexFile::from_path(&index_files.join(rel_path).join(entry.file_name()));
            match oldif {
                Ok(Ok(oldif)) if oldif == newif => {}
                _ => changes.push(IndexChange::AddFile(
                    rel_path.join(entry.file_name()),
                    newif,
                )),
            }
        }
    }
    Ok(())
}
