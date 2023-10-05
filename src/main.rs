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
    let changes = match perform_index_diff(&args.source, &args.index) {
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
        eprintln!("  >> add directory");
        eprintln!("  +  add/update file");
        eprintln!("  -  remove file");
        eprintln!(" [-] remove directory (and all contents!)");
        eprintln!("Press Enter to to apply these actions.");
        // apply changes
        if std::io::stdin().read_line(&mut String::new()).is_ok() {
            match apply_indexchanges(&args.source, &args.index, &args.target, &changes) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Failed to apply: {e}");
                    exit(30);
                }
            }
        }
    }
}
