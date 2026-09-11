use std::{
    sync::Arc,
    net::SocketAddrV4,
    mem, 
};

use tokio::{
    time::{Duration, Instant},
    sync::{Mutex, mpsc},
};
use tracing::{info, error};

use crate::{
    tracker::{AnnounceEvent, AnnounceError, Tracker},
    TorrentFile,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Announce(#[from] AnnounceError),
}

#[derive(Debug, Default)]
pub enum SessionStatus {
    #[default]
    Starting,
    Downloading,
    Stopped,
}

#[derive(Debug)]
pub struct SessionState {
    pub status: SessionStatus,
    pub peers: Vec<SocketAddrV4>,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub bytes_left: u64,
}

impl SessionState {
    pub fn new(peers: Vec<SocketAddrV4>, total_size: u64) -> Self {
        Self {
            status: SessionStatus::Starting,
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
    pub state: Arc<Mutex<SessionState>>,
    pub cmd_tx: mpsc::Sender<SessionCommand>,
    pub handle: tokio::task::JoinHandle<()>,
}

impl TorrentSession {
    pub async fn new(torrent: TorrentFile, peer_id: &[u8; 20], port: u16) -> Result<Self, Error> {
        let tracker = Tracker::new(torrent.announce.clone(), &torrent.info_hash, peer_id);

        info!("doing an initial announce");
        let response = tracker.announce(port, 0, 0, torrent.total_size, AnnounceEvent::Started).await?;
        let interval_duration = Duration::from_secs(response.interval);
        let peers = response.peers;
        info!(peers = ?peers, "received initial peers");
        
        let mut interval = tokio::time::interval_at(Instant::now() + interval_duration, interval_duration);

        let state = Arc::new(Mutex::new(SessionState::new(peers, torrent.total_size)));
        let _state = Arc::clone(&state);

        let (cmd_tx, mut cmd_rx) = mpsc::channel(8);

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

        Ok(Self {
            file: Arc::new(torrent),
            state,
            cmd_tx: cmd_tx.clone(),
            handle,
        })
    }
}