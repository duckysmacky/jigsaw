use std::{
    net::{Ipv4Addr, SocketAddrV4},
    rc::Rc,
};

use thiserror::Error;

use jigsaw_bencode::{
    types::{BencodeDict, BencodeElement, ByteString},
    parser::{self, BencodeParser},
    DecodeError, bencode_get,
};

use crate::util;

#[derive(Error, Debug)]
pub enum AnnounceError {
    #[error("Announce was declined by the tracker ({0})")]
    Failed(String),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Parse(#[from] parser::Error),
}

#[derive(Debug)]
pub struct AnnouceResponse {
    interval: u64,
    peers: Vec<SocketAddrV4>,
}

impl AnnouceResponse {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, AnnounceError> {
        let mut parser = BencodeParser::new(Rc::from(bytes));
        let bencode_element = parser.parse()?;
        
        match bencode_element {
            BencodeElement::Dict(dict) => Self::from_bencode(dict),
            _ => Err(DecodeError::WrongType("(initial value)".to_string()).into())
        }
    }

    pub fn from_bencode(dict: BencodeDict) -> Result<Self, AnnounceError> {
        if let Some(failure_reason) = bencode_get!(dict: optional "failure reason", String => |x: &ByteString| x.to_string())? {
            return Err(AnnounceError::Failed(failure_reason))
        }

        let interval = bencode_get!(dict: required "interval", Int => |x: &i64| *x as u64)?;
        let peers = bencode_get!(dict: required "peers", String => |byte_string: &ByteString| byte_string.bytes()
            .chunks_exact(6)
            .map(|bytes| {
                let ip = Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]);
                let port = u16::from_be_bytes([bytes[4], bytes[5]]);
                SocketAddrV4::new(ip, port)
            })
            .collect()
        )?;

        Ok(Self {
            interval,
            peers,
        })
    }
}

pub async fn announce(
    tracker_url: &str,
    info_hash: &[u8; 20],
    peer_id: &[u8; 20],
    port: u16,
    left: u64,
) -> Result<AnnouceResponse, AnnounceError> {
    let url = format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded=0&downloaded=0&left={}&compact=1&event=started",
        tracker_url,
        util::percent_encode(info_hash),
        util::percent_encode(peer_id),
        port,
        left,
    );

    let bytes = reqwest::get(&url)
        .await?
        .bytes()
        .await?;

    AnnouceResponse::from_bytes(bytes.to_vec())
}
