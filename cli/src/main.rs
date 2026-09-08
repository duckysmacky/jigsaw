mod cli;

use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;

use cli::{CliArgs, Commands};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = CliArgs::parse();

    match args.command {
        Some(Commands::Dump{ torrent_file, output_file, debug }) => {
            let display = torrent_file.display();

            if let Err(err) = dump_torrent_file(&torrent_file, &output_file, debug) {
                eprintln!("Unable to dump '{display}': {err}");
            }
        },
        None => todo!(),
    }
}

fn dump_torrent_file(input_path: &Path, output_path: &Option<PathBuf>, debug: bool) -> anyhow::Result<()> {
    let mut file = File::open(&input_path)?;
    let mut buf = Vec::new();

    file.read_to_end(&mut buf)?;

    match output_path {
        Some(output_path) => {
            let mut file = File::create(&output_path)
                .context("Unable to create the output file")?;
            
            println!("Dumping contents of '{}' to '{}'", input_path.display(), output_path.display());
            jigsaw_core::dump::dump_bencode(&mut file, buf, debug)?;
        },
        None => {
            let mut stdout = io::stdout();
            
            println!("Dumping contents of '{}':", input_path.display());
            jigsaw_core::dump::dump_bencode(&mut stdout, buf, debug)?;
        }
    }

    Ok(())
}
