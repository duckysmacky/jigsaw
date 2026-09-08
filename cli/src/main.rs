mod cli;

use std::{
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;
use tokio::{
    fs::File, io::{self, AsyncReadExt}
};

use jigsaw_core::{
    tracker::{Tracker, AnnounceEvent},
    session::SessionCommand,
    TorrentFile, TorrentClient,
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

            if let Err(err) = dump_torrent_file(&torrent_file, &output_file, debug).await {
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

async fn dump_torrent_file(input_path: &Path, output_path: &Option<PathBuf>, debug: bool) -> anyhow::Result<()> {
    let bytes = read_file(input_path).await?;

    match output_path {
        Some(output_path) => {
            let mut file = File::create(&output_path).await
                .context("Unable to create the output file")?;
            
            println!("Dumping contents of '{}' to '{}'", input_path.display(), output_path.display());
            jigsaw_core::dump::dump_bencode(&mut file, bytes, debug).await?;
        },
        None => {
            let mut stdout = io::stdout();
            
            println!("Dumping contents of '{}':", input_path.display());
            jigsaw_core::dump::dump_bencode(&mut stdout, bytes, debug).await?;
        }
    }

    Ok(())
}

async fn announce(client: TorrentClient, torrent_file: &Path) -> anyhow::Result<()> {
    let bytes = read_file(torrent_file).await?;
    let total_size = bytes.len() as u64;
    let torrent = TorrentFile::from_bytes(bytes)?;
    let tracker = Tracker::new(torrent.announce.clone(), &torrent.info_hash, client.peer_id());

    let response = tracker.announce(client.port(), 0, 0, total_size, AnnounceEvent::Started).await?;

    println!("Got announce response:");
    println!("{:#?}", response);

    Ok(())
}

async fn start(mut client: TorrentClient, torrent_file: &Path) -> anyhow::Result<()> {
    let bytes = read_file(torrent_file).await?;
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

async fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;
    Ok(buf)
}
