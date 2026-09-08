mod cli;

use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;

use jigsaw_core::{
    tracker::{Tracker, AnnounceEvent},
    client::{TorrentClient, SessionCommand},
    TorrentFile,
};

use cli::{CliArgs, Commands};
use tokio::io::AsyncBufReadExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = CliArgs::parse();
    let client = TorrentClient::new(8080);

    match args.command {
        Some(Commands::Dump{ torrent_file, output_file, debug }) => {
            let display = torrent_file.display();

            if let Err(err) = dump_torrent_file(&torrent_file, &output_file, debug) {
                eprintln!("Unable to dump '{display}': {err}");
            }
        },
        Some(Commands::Announce { torrent_file }) => {
            let display = torrent_file.display();

            if let Err(err) = announce(client, &torrent_file).await {
                eprintln!("Unable to announce for '{display}': {err}");
            }
        },
        Some(Commands::Start { torrent_file }) => {
            let display = torrent_file.display();

            if let Err(err) = start(client, &torrent_file).await {
                eprintln!("Unable to start torrent loop for '{display}': {err}");
            }
        }
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

async fn announce(client: TorrentClient, torrent_file: &Path) -> anyhow::Result<()> {
    let bytes = read_file(torrent_file)?;
    let total_size = bytes.len() as u64;
    let torrent = TorrentFile::from_bytes(bytes)?;
    let tracker = Tracker::new(torrent.announce.clone(), &torrent.info_hash, client.peer_id());

    let response = tracker.announce(client.port(), 0, 0, total_size, AnnounceEvent::Started).await?;

    println!("Got announce response:");
    println!("{:#?}", response);

    Ok(())
}

async fn start(mut client: TorrentClient, torrent_file: &Path) -> anyhow::Result<()> {
    let bytes = read_file(torrent_file)?;
    let total_size = bytes.len() as u64;
    let torrent = TorrentFile::from_bytes(bytes)?;

    let cmd_tx = client.start_session(torrent, total_size).await?;

    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim() == "stop" {
            cmd_tx.send(SessionCommand::Stop).await?;
            break;
        }
    }
    
    client.await_session().await?;

    Ok(())
}

fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(&path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
