use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Parse a .torrent file and dump its contents
    Dump {
        /// Path to the .torrent file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Display in debug form (with types)
        #[arg(short, long)]
        debug: bool,
    },
}

