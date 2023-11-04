use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, version)]
pub struct Args {
    /// the data to be backed up
    #[arg()]
    pub source: PathBuf,
    /// the index used to determine which files have been modified
    #[arg()]
    pub index: PathBuf,
    /// where your backup will be stored
    #[arg()]
    pub target: Option<PathBuf>,
    /// directories to ignore.
    /// can be paths relative to <source> (like backups/) or paths starting with <source> (like my_source/backups/).
    /// if <index> starts with <source>, it is automatically ignored and doesn't need to be specified.
    #[arg(long)]
    pub ignore: Vec<PathBuf>,
    /// don't ask for confirmation, just apply the changes.
    #[arg(long)]
    pub noconfirm: bool,
}
