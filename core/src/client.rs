use tokio::sync::mpsc;
use tracing::error;

use crate::{
    session::{self, TorrentSession, SessionCommand},
    TorrentFile,
};

/// Client ID, which is based on the name of "Jigsaw"
const CLIENT_ID: &'static str = "JS";
/// Client version, which is made up of `major.minor.patch<cycle>`,
/// where `cycle` is a letter for "beta", "alpha", "dev", "release", etc.
/// 
/// E.g., 1.0.1r (release)
const CLIENT_VERSION: [u8; 4] = [b'0', b'0', b'1', b'd'];

pub struct TorrentClient {
    peer_id: [u8; 20],
    port: u16,
    // TODO: turn from single-session to multi-session
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

    pub async fn start_session(&mut self, torrent: TorrentFile, total_size: u64) -> Result<mpsc::Sender<SessionCommand>, session::Error> {
        let session = TorrentSession::new(torrent, total_size, &self.peer_id, self.port).await?;

        let cmd_tx = session.cmd_tx.clone();
        self.session = Some(session);

        Ok(cmd_tx)
    }

    pub async fn await_session(self) -> Result<(), session::Error> {
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