mod cli;

use std::{
    fs::File,
    io::Read,
    path::Path,
};

use clap::Parser;

use jigsaw_core::bencode::BencodeParser;

use cli::{CliArgs, Commands};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = CliArgs::parse();

    match args.command {
        Some(Commands::Dump{ file, debug }) => {
            let display = file.display();

            if let Err(err) = dump_torrent_file(&file, debug) {
                eprintln!("Unable to dump '{display}': {err}");
            }
        },
        None => todo!(),
    }
}

fn dump_torrent_file(path: &Path, debug: bool) -> anyhow::Result<()> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();

    file.read_to_end(&mut buf)?;

    let mut parser = BencodeParser::new(&buf);
    let parsed_file = parser.parse()?;

    println!("Dumping contents of '{}':", path.display());
    if debug {
        println!("{:#?}", parsed_file);
    } else {
        println!("{}", parsed_file);
    }

    Ok(())
}
