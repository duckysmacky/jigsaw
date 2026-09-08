/// Client ID, which is based on the name of "Jigsaw"
const CLIENT_ID: &'static str = "JS";
/// Client version, which is made up of `major.minor.patch<cycle>`,
/// where `cycle` is a letter for "beta", "alpha", "dev", "release", etc.
/// 
/// E.g., 1.0.1r (release)
const CLIENT_VERSION: [u8; 4] = [b'0', b'0', b'1', b'd'];

pub struct TorrentClient {
    peer_id: [u8; 20],
}

impl TorrentClient {
    pub fn new() -> Self {
        Self {
            peer_id: generate_peer_id(),
        }
    }

    pub fn peer_id(&self) -> &[u8; 20] {
        &self.peer_id
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