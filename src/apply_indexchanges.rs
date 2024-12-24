use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{indexchanges::IndexChange, repr_file::ReprFile};

/// Only errors that happen when writing to the index are immediately returned.
/// Other errors are logged to stderr and the failed change will not be saved to the index,
/// so the next backup will try again.
pub fn apply_indexchanges(
    source: &Path,
    index: &Path,
    target: &Option<PathBuf>,
    changes: &Vec<IndexChange>,
    gib_total: Option<f64>,
) -> io::Result<()> {
    let o = apply_indexchanges_int(source, index, target, changes, gib_total);
    eprintln!();
    o
}

fn eprint_constants(changes_total: usize, gib_total: f64) -> (usize, usize, usize) {
    let changes_len_width = changes_total.to_string().len();
    let gib_len_width = format!("{gib_total:.1}").len();
    let prog_width =
        (80usize /* term min width */ - 3 /* progress bar [, >, ] */ - 6/* slash, space, pipe, space, slash, space */)
            .saturating_sub(changes_len_width + changes_len_width);
    (prog_width, changes_len_width, gib_len_width)
}
fn eprint_status(
    changes_applied: usize,
    changes_total: usize,
    gib_transferred: f64,
    gib_total: f64,
    prog_width: usize,
    changes_len_width: usize,
    gib_len_width: usize,
) {
    let leftpad_min = prog_width.min(
        (prog_width as f64
            * f64::min(
                changes_applied as f64 / changes_total as f64,
                gib_transferred / gib_total,
            ))
        .round() as usize,
    );
    let leftpad_max = prog_width.min(
        (prog_width as f64
            * f64::max(
                changes_applied as f64 / changes_total as f64,
                gib_transferred / gib_total,
            ))
        .round() as usize,
    );
    let changes_applied = changes_applied.to_string();
    let changes_pad = " ".repeat(changes_len_width - changes_applied.len());
    let gib_transferred = format!("{gib_transferred:.1}");
    let gib_pad = " ".repeat(gib_len_width - gib_transferred.len());
    let rightpad = prog_width - leftpad_max;
    let completed_prog_min = "=".repeat(leftpad_min);
    let completed_prog_max = "-".repeat(leftpad_max - leftpad_min);
    let pending_prog = " ".repeat(rightpad);
    eprint!(
        "\r{changes_pad}{changes_applied}/{changes_total} | {gib_pad}{gib_transferred}/{gib_total:.1}GiB [{completed_prog_min}{completed_prog_max}>{pending_prog}]",

    );
}

pub fn apply_indexchanges_int(
    source: &Path,
    index: &Path,
    target: &Option<PathBuf>,
    changes: &Vec<IndexChange>,
    gib_total: Option<f64>,
) -> io::Result<()> {
    let changes_total = changes.len();
    let gib_total = gib_total.unwrap_or_else(|| {
        changes
            .iter()
            .filter_map(|c| {
                if let IndexChange::AddFile(_, i) = c {
                    Some(i.size as f64 / (1024 * 1024 * 1024) as f64)
                } else {
                    None
                }
            })
            .sum()
    });
    let (prog_width, changes_len_width, gib_len_width) = eprint_constants(changes.len(), gib_total);
    let mut gib_transferred = 0.0;
    eprint_status(
        0,
        changes_total,
        gib_transferred,
        gib_total,
        prog_width,
        changes_len_width,
        gib_len_width,
    );
    for (i, change) in changes.iter().enumerate() {
        match change {
            IndexChange::AddDir(dir, _) => {
                let ok = if let Some(target) = target {
                    let t = target.join(dir);
                    if let Err(e) = fs::create_dir_all(&t) {
                        eprintln!("\n[warn] couldn't create directory {t:?}: {e}");
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if ok {
                    fs::create_dir_all(&index.join(dir))?;
                }
            }
            IndexChange::AddFile(file, index_file) => {
                gib_transferred += index_file.size as f64 / (1024 * 1024 * 1024) as f64;
                let ok = if let Some(target) = target {
                    let s = source.join(file);
                    let t = target.join(file);
                    if let Err(e) = fs::copy(&s, &t) {
                        eprintln!("\n[warn] couldn't copy file from {s:?} to {t:?}: {e}");
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if ok {
                    fs::write(&index.join(file), index_file.save())?;
                }
            }
            IndexChange::RemoveFile(file) => {
                let i = index.join(file);
                let ok = if let Some(target) = target {
                    let t = target.join(file);
                    if let Err(e) = fs::remove_file(&t) {
                        eprintln!("\n[warn] couldn't remove file {t:?}, keeping index file {i:?}: {e:?}\n     If this error keeps appearing, check if the file was deleted on the target system but still exists in the index. if yes, consider manually deleting it.");
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if ok {
                    fs::remove_file(i)?;
                }
            }
            IndexChange::RemoveDir(dir) => {
                let i = index.join(dir);
                let ok = if let Some(target) = target {
                    let t = target.join(dir);
                    if let Err(e) = fs::remove_dir_all(&t) {
                        eprintln!("\n[warn] couldn't remove directory {t:?}, keeping index files under {i:?}: {e:?}\n     If this error keeps appearing, check if the directory was deleted on the target system but still exists in the index. if yes, consider manually deleting it.");
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if ok {
                    fs::remove_dir_all(i)?;
                }
            }
        }
        {
            eprint_status(
                i + 1,
                changes.len(),
                gib_transferred,
                gib_total,
                prog_width,
                changes_len_width,
                gib_len_width,
            );
        }
    }
    Ok(())
}
