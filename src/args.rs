use std::path::PathBuf;

use clap::Parser;

use crate::update_index::Settings;

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

    #[arg(long)]
    pub ignore: Option<PathBuf>,

    #[command(flatten)]
    pub settings: Settings,
}
