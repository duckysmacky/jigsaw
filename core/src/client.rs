use std::{
    net::SocketAddrV4, 
    sync::Arc,
    mem, 
};

use tokio::{
    time::{Duration, Instant},
    sync::{Mutex, mpsc},
};
use tracing::{info, error};

use crate::{
    tracker::{AnnounceError, AnnounceEvent, Tracker},
    TorrentFile,
};

/// Client ID, which is based on the name of "Jigsaw"
const CLIENT_ID: &'static str = "JS";
/// Client version, which is made up of `major.minor.patch<cycle>`,
/// where `cycle` is a letter for "beta", "alpha", "dev", "release", etc.
/// 
/// E.g., 1.0.1r (release)
const CLIENT_VERSION: [u8; 4] = [b'0', b'0', b'1', b'd'];

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Announce(#[from] AnnounceError),
}

#[derive(Debug, Default)]
pub enum TorrentStatus {
    #[default]
    Starting,
    Downloading,
    Stopped,
}

#[derive(Debug)]
pub struct TorrentState {
    pub status: TorrentStatus,
    pub peers: Vec<SocketAddrV4>,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub bytes_left: u64,
}

impl TorrentState {
    pub fn new(peers: Vec<SocketAddrV4>, total_size: u64) -> Self {
        Self {
            status: TorrentStatus::Starting,
            peers,
            bytes_uploaded: 0,
            bytes_downloaded: 0,
            bytes_left: total_size,
        }
    }
}

#[derive(Debug)]
pub enum SessionCommand {
    Stop,
}

#[derive(Debug)]
pub struct TorrentSession {
    pub file: Arc<TorrentFile>,
    pub state: Arc<Mutex<TorrentState>>,
    pub cmd_tx: mpsc::Sender<SessionCommand>,
    pub handle: tokio::task::JoinHandle<()>,
}

pub struct TorrentClient {
    peer_id: [u8; 20],
    port: u16,
    session: Option<TorrentSession>,
}

impl TorrentClient {
    pub fn new(port: u16) -> Self {
        Self {
            peer_id: generate_peer_id(),
            port,
            session: None,
        }
    }

    pub async fn start_session(&mut self, torrent: TorrentFile, total_size: u64) -> Result<mpsc::Sender<SessionCommand>, Error> {
        let tracker = Tracker::new(torrent.announce.clone(), &torrent.info_hash, &self.peer_id);

        info!("doing an initial announce");
        let response = tracker.announce(self.port, 0, 0, total_size, AnnounceEvent::Started).await?;
        let interval_duration = Duration::from_secs(response.interval);
        let peers = response.peers;
        info!(peers = ?peers, "received initial peers");
        
        let mut interval = tokio::time::interval_at(Instant::now() + interval_duration, interval_duration);

        let state = Arc::new(Mutex::new(TorrentState::new(peers, total_size)));
        let _state = Arc::clone(&state);

        let (cmd_tx, mut cmd_rx) = mpsc::channel(8);

        let port = self.port;
        let handle = tokio::spawn(async move {
            info!("starting torrent session");

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let state = _state.lock().await;
                        let uploaded = state.bytes_uploaded;
                        let downloaded = state.bytes_downloaded;
                        let left = state.bytes_left;
                        mem::drop(state);
                        
                        info!("refreshing peer list");

                        match tracker.announce(port, uploaded, downloaded, left, AnnounceEvent::Refresh).await {
                            Ok(response) => {
                                info!(peers = ?response.peers, "received new peers");

                                let mut state = _state.lock().await;
                                state.peers = response.peers;
                            },
                            Err(err) => {
                                error!(error = err.to_string(), "unable to get tracker response");
                            }
                        }
                    },
                    Some(cmd) = cmd_rx.recv() => match cmd {
                        SessionCommand::Stop => {
                            info!("stop signal recevied, ending the session");
                            break;
                        },
                    },
                    // other events... will be aded later
                }
            }
        });

        self.session = Some(TorrentSession {
            file: Arc::new(torrent),
            state,
            cmd_tx: cmd_tx.clone(),
            handle,
        });

        Ok(cmd_tx)
    }

    pub async fn await_session(self) -> Result<(), Error> {
        if let Some(session) = self.session {
            if let Err(err) = session.handle.await {
                error!(error = err.to_string(), "unable to await session")
            }
        }

        Ok(())
    }

    pub fn peer_id(&self) -> &[u8; 20] {
        &self.peer_id
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

fn generate_peer_id() -> [u8; 20] {
    let mut peer_id = [0u8; 20];

    peer_id[0] = b'-';
    peer_id[1..3].copy_from_slice(CLIENT_ID.as_bytes());
    peer_id[3..7].copy_from_slice(&CLIENT_VERSION);
    peer_id[7] = b'-';
    rand::fill(&mut peer_id[8..20]);

    peer_id
}