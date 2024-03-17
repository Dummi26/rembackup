use std::process::exit;

use clap::Parser;

use crate::{
    apply_indexchanges::apply_indexchanges, indexchanges::IndexChange,
    update_index::perform_index_diff,
};

mod apply_indexchanges;
mod args;
mod indexchanges;
mod indexfile;
mod repr_file;
mod update_index;

fn main() {
    // get args
    let args = args::Args::parse();
    // index diff
    eprintln!("performing index diff...");
    let source = &args.source;
    let index = &args.index;
    let ignore_subdirs = args
        .ignore
        .iter()
        .map(|path| path.strip_prefix(source).unwrap_or(path))
        .collect();
    let changes = match perform_index_diff(source, index, ignore_subdirs, &args.settings) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to generate index diff:\n    {e}");
            exit(20);
        }
    };
    if changes.is_empty() {
        eprintln!("done! found no changes.");
    } else {
        eprintln!("done! found {} changes:", changes.len());
        // display the changes
        for change in &changes {
            match change {
                IndexChange::AddDir(v) => eprintln!("  >> {}", v.display()),
                IndexChange::AddFile(v, _) => eprintln!("  +  {}", v.display()),
                IndexChange::RemoveFile(v) => eprintln!("  -  {}", v.display()),
                IndexChange::RemoveDir(v) => eprintln!(" [-] {}", v.display()),
            }
        }
        eprintln!(" - - - - -");
        eprintln!(
            "  >> add directory | {}x",
            changes
                .iter()
                .filter(|c| matches!(c, IndexChange::AddDir(..)))
                .count()
        );
        eprintln!(
            "  +  add/update file | {}x",
            changes
                .iter()
                .filter(|c| matches!(c, IndexChange::AddFile(..)))
                .count()
        );
        eprintln!(
            "  -  remove file | {}x",
            changes
                .iter()
                .filter(|c| matches!(c, IndexChange::RemoveFile(..)))
                .count()
        );
        eprintln!(
            " [-] remove directory (and all contents!) | {}x",
            changes
                .iter()
                .filter(|c| matches!(c, IndexChange::RemoveDir(..)))
                .count()
        );
        // apply changes after confirming
        if !args.noconfirm {
            let mut line = String::new();
            loop {
                if args.target.is_none() {
                    eprintln!("[WARN] You didn't set a `target` directory!\n[WARN] Be careful not to update your index without actually applying the changes to the `target` filesystem!\nType 'Ok' and press enter to continue.");
                } else {
                    eprintln!("Exclude unwanted directories/files using --ignore,\nor press enter to apply the changes.");
                }
                line.clear();
                std::io::stdin().read_line(&mut line).unwrap();
                let line = line.trim().to_lowercase();
                if line == "exit" {
                    return;
                } else if args.target.is_some() || line == "ok" {
                    break;
                }
            }
        }
        match apply_indexchanges(&args.source, &args.index, &args.target, &changes) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Failed to apply: {e}");
                exit(30);
            }
        }
    }
}
