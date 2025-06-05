use std::{
    fs,
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
    changes: &[IndexChange],
    gib_total: Option<f64>,
) -> usize {
    // do symlinks last, as they cd, which can fail,
    // and if it does, it would be fatal and stop the backup.
    let (mut changes, symlink_additions) =
        changes.into_iter().partition::<Vec<_>, _>(|c| match c {
            IndexChange::AddDir(..)
            | IndexChange::AddFile(..)
            | IndexChange::RemoveFile(..)
            | IndexChange::RemoveDir(..) => true,
            IndexChange::AddSymlink(..) => false,
        });
    changes.extend(symlink_additions);

    let mut failures = changes.len();
    apply_indexchanges_int(source, index, target, &changes, gib_total, &mut failures);
    eprintln!();
    failures
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
    changes: &[&IndexChange],
    gib_total: Option<f64>,
    failures: &mut usize,
) {
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
    let (prog_width, changes_len_width, gib_len_width) = eprint_constants(changes_total, gib_total);
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
            IndexChange::AddDir(dir, make_new, _) => {
                if *make_new {
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
                        *failures -= 1;
                        let t = index.join(dir);
                        if let Err(e) = fs::create_dir_all(&t) {
                            eprintln!("\n[warn] couldn't create index directory {t:?}: {e}");
                        }
                    }
                } else {
                    *failures -= 1;
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
                    *failures -= 1;
                    let t = index.join(file);
                    if let Err(e) = fs::write(&t, index_file.save()) {
                        eprintln!("\n[warn] couldn't save index file {t:?}: {e}");
                    }
                }
            }
            IndexChange::AddSymlink(file, link_target) => {
                let cwd = match std::env::current_dir() {
                    Ok(cwd) => cwd,
                    Err(e) => {
                        eprintln!("\n[err] fatal: couldn't get cwd: {e}");
                        return;
                    }
                };
                let ok = if let Some(target) = target {
                    let t = target.join(file);
                    if let Some(p) = t.parent() {
                        let t = t.file_name().expect("a file should always have a filename");
                        if let Err(e) = std::env::set_current_dir(&p) {
                            eprintln!("\n[warn] couldn't cd to {p:?}: {e}");
                            false
                        } else {
                            let _ = std::fs::remove_file(t);
                            if let Err(e) = std::os::unix::fs::symlink(&link_target, t) {
                                eprintln!(
                                    "\n[warn] couldn't set file {t:?} to be a symlink to {link_target:?}: {e}"
                                );
                                false
                            } else {
                                if let Err(e) = std::env::set_current_dir(&cwd) {
                                    eprintln!("\n[err] fatal: couldn't cd back to {cwd:?}: {e}");
                                    return;
                                }
                                true
                            }
                        }
                    } else {
                        eprintln!("\n[warn] symlink path was empty");
                        false
                    }
                } else {
                    true
                };
                if ok {
                    *failures -= 1;
                    let index_file = index.join(file);
                    if let Some(p) = index_file.parent() {
                        if let Err(e) = std::env::set_current_dir(&p) {
                            eprintln!("\n[warn] couldn't cd to {p:?}: {e}");
                        } else {
                            if let Err(e) = std::os::unix::fs::symlink(
                                link_target,
                                index_file
                                    .file_name()
                                    .expect("a file should always have a filename"),
                            ) {
                                eprintln!(
                                    "\n[warn] couldn't set index file {index_file:?} to be a symlink to {link_target:?}: {e}"
                                );
                            }
                        }
                    } else {
                        eprintln!(
                            "\n[warn] couldn't get parent for index file's path, so could not create the symlink"
                        );
                    }
                }
                if let Err(e) = std::env::set_current_dir(&cwd) {
                    eprintln!("\n[err] fatal: couldn't cd back to {cwd:?}: {e}");
                    return;
                }
            }
            IndexChange::RemoveFile(file) => {
                let i = index.join(file);
                let ok = if let Some(target) = target {
                    let t = target.join(file);
                    if let Err(e) = fs::remove_file(&t) {
                        eprintln!(
                            "\n[warn] couldn't remove file {t:?}, keeping index file {i:?}: {e:?}\n     If this error keeps appearing, check if the file was deleted on the target system but still exists in the index. if yes, consider manually deleting it."
                        );
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if ok {
                    *failures -= 1;
                    if let Err(e) = fs::remove_file(&i) {
                        eprintln!("\n[warn] couldn't remove index file {i:?}: {e:?}");
                    }
                }
            }
            IndexChange::RemoveDir(dir) => {
                let i = index.join(dir);
                let ok = if let Some(target) = target {
                    let t = target.join(dir);
                    if let Err(e) = fs::remove_dir_all(&t) {
                        eprintln!(
                            "\n[warn] couldn't remove directory {t:?}, keeping index files under {i:?}: {e:?}\n     If this error keeps appearing, check if the directory was deleted on the target system but still exists in the index. if yes, consider manually deleting it."
                        );
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if ok {
                    *failures -= 1;
                    if let Err(e) = fs::remove_dir_all(&i) {
                        eprintln!("\n[warn] couldn't remove index directory {i:?}: {e:?}");
                    }
                }
            }
        }
        {
            eprint_status(
                i + 1,
                changes_total,
                gib_transferred,
                gib_total,
                prog_width,
                changes_len_width,
                gib_len_width,
            );
        }
    }
}
