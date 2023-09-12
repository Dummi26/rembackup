use std::{fs, io, path::Path};

use crate::{indexchanges::IndexChange, repr_file::ReprFile};

/// Only errors that happen when writing to the index are immediately returned.
/// Other errors are logged to stderr and the failed change will not be saved to the index,
/// so the next backup will try again.
pub fn apply_indexchanges(
    source: &Path,
    index: &Path,
    target: &Path,
    changes: &Vec<IndexChange>,
) -> io::Result<()> {
    let o = apply_indexchanges_int(source, index, target, changes);
    eprintln!();
    o
}
pub fn apply_indexchanges_int(
    source: &Path,
    index: &Path,
    target: &Path,
    changes: &Vec<IndexChange>,
) -> io::Result<()> {
    let len_width = changes.len().to_string().len();
    let width = 80 - 3 - 2 - len_width - len_width;
    eprint!(
        "{}0/{} [>{}]",
        " ".repeat(len_width - 1),
        changes.len(),
        " ".repeat(width)
    );
    for (i, change) in changes.iter().enumerate() {
        match change {
            IndexChange::AddDir(dir) => {
                let t = target.join(dir);
                if let Err(e) = fs::create_dir(&t) {
                    eprintln!("\n[warn] couldn't create directory {t:?}: {e}");
                } else {
                    fs::create_dir(&index.join(dir))?;
                }
            }
            IndexChange::AddFile(file, index_file) => {
                let s = source.join(file);
                let t = target.join(file);
                if let Err(e) = fs::copy(&s, &t) {
                    eprintln!("\n[warn] couldn't copy file from {s:?} to {t:?}: {e}");
                }
                fs::write(&index.join(file), index_file.save())?;
            }
        }
        {
            let i = i + 1;
            let leftpad = width * i / changes.len();
            let rightpad = width - leftpad;
            let prognum = i.to_string();
            eprint!(
                "\r{}{}/{} [{}>{}]",
                " ".repeat(len_width - prognum.len()),
                prognum,
                changes.len(),
                "-".repeat(leftpad),
                " ".repeat(rightpad)
            );
        }
    }
    Ok(())
}
