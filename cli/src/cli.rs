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
    /// 
    /// By default will dump to `stdout`, but an output file can be specified
    /// with the `--file` (`-f`) argument
    Dump {
        /// Path to the .torrent file
        #[arg(value_name = "TORRENT_FILE")]
        torrent_file: PathBuf,
        
        /// Path to the optional output file
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output_file: Option<PathBuf>,

        /// Display in debug form (with types)
        #[arg(short, long)]
        debug: bool,
    },
}

