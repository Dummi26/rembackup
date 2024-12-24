use std::process::exit;

use clap::Parser;

use crate::{
    apply_indexchanges::apply_indexchanges, config::Ignore, indexchanges::IndexChange,
    update_index::perform_index_diff,
};

mod apply_indexchanges;
mod args;
mod config;
mod indexchanges;
mod indexfile;
mod repr_file;
mod update_index;

const EXIT_IGNORE_FAILED: u8 = 200;
const EXIT_DIFF_FAILED: u8 = 20;
const EXIT_APPLY_FAILED: u8 = 30;

fn main() {
    // get args
    let args = args::Args::parse();
    // index diff
    eprintln!("performing index diff...");
    let cwd = match std::env::current_dir() {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("[WARN] Couldn't get current directory (CWD): {e}");
            None
        }
    };
    let source = if args.source.is_absolute() {
        args.source.clone()
    } else {
        cwd.as_ref()
            .expect("tried to use a relative path when there is no valid CWD")
            .join(&args.source)
    };
    let index = if args.index.is_absolute() {
        args.index.clone()
    } else {
        cwd.as_ref()
            .expect("tried to use a relative path when there is no valid CWD")
            .join(&args.index)
    };
    let target = args.target.as_ref().map(|target| {
        if target.is_absolute() {
            target.clone()
        } else {
            cwd.as_ref()
                .expect("tried to use a relative path when there is no valid CWD")
                .join(target)
        }
    });
    let ignore = if let Some(path) = &args.ignore {
        match std::fs::read_to_string(path) {
            Ok(text) => match Ignore::parse(&text) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Couldn't parse ignore-file {path:?}: {e}");
                    exit(EXIT_IGNORE_FAILED as _);
                }
            },
            Err(e) => {
                eprintln!("Couldn't load ignore-file {path:?}: {e}");
                exit(EXIT_IGNORE_FAILED as _);
            }
        }
    } else {
        Ignore(vec![])
    };
    let (total_size, changes) = match perform_index_diff(
        &source,
        &index,
        target.as_ref().map(|v| v.as_path()),
        ignore,
        &args.settings,
        if args.settings.dont_sort {
            None
        } else {
            Some(!args.settings.smallest_first)
        },
    ) {
        Ok(c) => c,
        Err((what, path, err)) => {
            eprintln!(
                "Failed to generate index diff:\n    {what}\n    {}\n    {err}",
                path.to_string_lossy()
            );
            exit(EXIT_DIFF_FAILED as _);
        }
    };
    if changes.is_empty() {
        eprintln!("done! found no changes.");
    } else {
        eprintln!("done! found {} changes:", changes.len());
        // display the changes
        if args.settings.dont_reverse_output {
            for change in &changes {
                show_change(change, false);
            }
        } else {
            for change in changes.iter().rev() {
                show_change(change, true);
            }
        }
        fn show_change(change: &IndexChange, rev: bool) {
            match change {
                IndexChange::AddDir(v, s) => {
                    let mut path_str = v.display().to_string();
                    if !path_str.ends_with(['/', '\\']) {
                        path_str.push('/');
                    }
                    eprintln!(
                        "{}>>  {}    [{:.2} GiB]",
                        if rev { "^" } else { "v" },
                        path_str,
                        *s as f64 / (1024 * 1024 * 1024) as f64
                    );
                }
                IndexChange::AddFile(v, f) => eprintln!(
                    "  +  {}    ({:.3} GiB)",
                    v.display(),
                    f.size as f64 / (1024 * 1024 * 1024) as f64
                ),
                IndexChange::RemoveFile(v) => eprintln!("  -  {}", v.display()),
                IndexChange::RemoveDir(v) => {
                    let mut path_str = v.display().to_string();
                    if !path_str.ends_with(['/', '\\']) {
                        path_str.push('/');
                    }
                    eprintln!(" [-] {}", path_str);
                }
            }
        }
        eprintln!(" - - - - -");
        let add_dir_count = changes
            .iter()
            .filter(|c| matches!(c, IndexChange::AddDir(..)))
            .count();
        eprintln!(
            " {}>> add directory | {add_dir_count}x",
            if args.settings.dont_reverse_output {
                "v"
            } else {
                "^"
            }
        );
        let add_file_count = changes
            .iter()
            .filter_map(|c| {
                if let IndexChange::AddFile(_, f) = c {
                    Some(f)
                } else {
                    None
                }
            })
            .count();
        let add_file_total_size_gib = total_size as f64 / (1024 * 1024 * 1024) as f64;
        eprintln!("  +  add/update file | {add_file_count}x ({add_file_total_size_gib:.1} GiB)");
        let remove_file_count = changes
            .iter()
            .filter(|c| matches!(c, IndexChange::RemoveFile(..)))
            .count();
        eprintln!("  -  remove file | {}x", remove_file_count);
        let remove_dir_count = changes
            .iter()
            .filter(|c| matches!(c, IndexChange::RemoveDir(..)))
            .count();
        eprintln!(" [-] remove directory (and all contents!) | {remove_dir_count}x");
        // apply changes after confirming
        if !args.noconfirm {
            loop {
                if args.target.is_none() {
                    eprintln!("[WARN] You didn't set a `target` directory!\n[WARN] Be careful not to update your index without actually applying the changes to the `target` filesystem!\nType 'Ok' and press enter to continue.");
                } else {
                    eprintln!("Exclude unwanted directories/files using --ignore,\nor press enter to apply the changes.");
                }
                let line = if let Some(Ok(v)) = std::io::stdin().lines().next() {
                    v
                } else {
                    return;
                };
                let line = line.trim().to_lowercase();
                if line == "exit" {
                    return;
                } else if args.target.is_some() || line == "ok" {
                    break;
                }
            }
        }
        match apply_indexchanges(
            &args.source,
            &args.index,
            &args.target,
            &changes,
            Some(add_file_total_size_gib),
        ) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Failed to apply: {e}");
                exit(EXIT_APPLY_FAILED as _);
            }
        }
    }
}
