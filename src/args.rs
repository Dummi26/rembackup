use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, version)]
pub struct Args {
    #[arg()]
    pub source: PathBuf,
    #[arg()]
    pub index: PathBuf,
    #[arg()]
    pub target: PathBuf,
}
