use std::path::PathBuf;

use clap::Parser;

use crate::update_index::Settings;

/// rembackup,
/// a simple backup tool for local or remote backups.
/// run with --help for more help.
///
/// rembackup copies files from <source> to <target> using and storing information in <index>.
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
    /// don't ask for confirmation, just apply the changes.
    #[arg(long)]
    pub noconfirm: bool,

    /// the file in which you specified what files/directories should be ignored
    #[arg(long)]
    pub ignore: Option<PathBuf>,

    #[command(flatten)]
    pub settings: Settings,
}
