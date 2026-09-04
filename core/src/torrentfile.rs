use std::path::PathBuf;

use jigsaw_bencode::BencodeElement;
use thiserror::Error;

use crate::bencode::BencodeDict;

// TODO: bittorrent v2 does things differently

#[derive(Error, Debug)]
pub enum StructureError {
    #[error("Field '{0}' is of wrong type.")]
    WrongType(String),
    #[error("Missing required key '{0}'.")]
    RequiredKeyMissing(String),
    #[error("Pieces bytestring length isn't divisible by 20.")]
    PiecesBytesLengthError,
    #[error("Length value is negative.")]
    NegativeLength,
}

#[derive(Debug)]
pub struct TorrentFile {
    pub announce: String,
    pub info: Info,

    // TODO: add announce-list at some point

    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub creation_date: Option<u64>,
}

impl TorrentFile {
    pub fn from_bencoded(dict: BencodeDict) -> Result<Self, StructureError> {
        let announce = match dict.get(&"announce".into()) {
            Some(BencodeElement::ByteString(announce)) => announce.to_string(),
            Some(_) => return Err(StructureError::WrongType("announce".to_string())),
            None => return Err(StructureError::RequiredKeyMissing("announce".to_string())),
        };

        let info = match dict.get(&"info".into()) {
            Some(BencodeElement::Dict(info_dict)) => Info::from_bencoded(info_dict)?,
            Some(_) => return Err(StructureError::WrongType("info".to_string())),
            None => return Err(StructureError::RequiredKeyMissing("info".to_string()))
        };

        let comment = match dict.get(&"comment".into()) {
            Some(BencodeElement::ByteString(comment)) => Some(comment.to_string()),
            Some(_) => return Err(StructureError::WrongType("comment".to_string())),
            None => None,
        };

        let created_by = match dict.get(&"created by".into()) {
            Some(BencodeElement::ByteString(created_by)) => Some(created_by.to_string()),
            Some(_) => return Err(StructureError::WrongType("created by".to_string())),
            None => None,
        };

        let creation_date = match dict.get(&"creation date".into()) {
            Some(BencodeElement::Int(creation_date)) => Some(*creation_date as u64),
            Some(_) => return Err(StructureError::WrongType("creation date".to_string())),
            None => None,
        };

        return Ok(Self {
            announce,
            info,
            comment,
            created_by,
            creation_date
        });
    }
}

#[derive(Debug)]
pub struct Info {
    pub name: String,
    pub file: FileMode,
    pub piece_length: u32,
    pub pieces_hashes: Vec<[u8; 20]>,
}

impl Info {
    pub fn from_bencoded(info_dict: &BencodeDict) -> Result<Self, StructureError> {
        // either one or the other should be present
        if !info_dict.contains_key(&"files".into()) && !info_dict.contains_key(&"length".into()) {
            return Err(StructureError::RequiredKeyMissing("files/length".to_string()));
        }

        let name = match info_dict.get(&"name".into()) {
            Some(BencodeElement::ByteString(name)) => name.to_string(),
            Some(_) => return Err(StructureError::WrongType("name".to_string())),
            None => return Err(StructureError::RequiredKeyMissing("name".to_string())),
        };

        let file = if info_dict.contains_key(&"files".into()) {
            let files = match &info_dict[&"files".into()] {
                BencodeElement::List(files_list) => {
                    let mut file_entries: Vec<FileEntry> = Vec::new();
                    for entry_elem in files_list {
                        match entry_elem {
                            BencodeElement::Dict(entry_dict) => {
                                let length = match entry_dict.get(&"length".into()) {
                                    Some(BencodeElement::Int(len)) => *len as u64,
                                    Some(_) => return Err(StructureError::WrongType("length".to_string())),
                                    None => return Err(StructureError::RequiredKeyMissing("length".to_string())),
                                };

                                let path = match entry_dict.get(&"path".into()) {
                                    Some(BencodeElement::List(path_list)) => {
                                        let mut path_parts = PathBuf::new();
                                        for path_part in path_list {
                                            match path_part {
                                                BencodeElement::ByteString(part) => path_parts.push(part.to_string()),
                                                _ => return Err(StructureError::WrongType("(path part)".to_string())),
                                            }
                                        }
                                        path_parts
                                    },
                                    Some(_) => return Err(StructureError::WrongType("path".to_string())),
                                    None => return Err(StructureError::RequiredKeyMissing("path".to_string())),
                                };

                                file_entries.push(FileEntry { length, path });
                            },
                            _ => return Err(StructureError::WrongType("(file entry)".to_string()))
                        }
                    }
                    file_entries
                },
                _ => return Err(StructureError::WrongType("files".to_string()))
            };

            FileMode::MultipleFiles { files }
        } else {
            let length = match info_dict[&"length".into()] {
                BencodeElement::Int(len) => {
                    if len < 0 { return Err(StructureError::NegativeLength) }
                    len as u64
                },
                _ => return Err(StructureError::WrongType("length".to_string()))
            };

            FileMode::SingleFile { length }
        };

        let piece_length = match info_dict.get(&"piece length".into()) {
            Some(BencodeElement::Int(piece_len)) => {
                if *piece_len < 0 { return Err(StructureError::NegativeLength) }
                *piece_len as u32
            }
            Some(_) => return Err(StructureError::WrongType("piece length".to_string())),
            None => return Err(StructureError::RequiredKeyMissing("piece length".to_string())),
        };

        let pieces_hashes = match info_dict.get(&"pieces".into()) {
            Some(BencodeElement::ByteString(bytes)) => {
                if bytes.len() % 20 != 0 {
                    return Err(StructureError::PiecesBytesLengthError);
                }
                // unwrap will not fail, the chunk size is guaranteed.
                bytes.inner()
                    .chunks_exact(20)
                    .map(|ch| ch.try_into().unwrap())
                    .collect::<Vec<[u8; 20]>>()
            },
            Some(_) => return Err(StructureError::WrongType("pieces".to_string())),
            None => return Err(StructureError::RequiredKeyMissing("pieces".to_string())),
        };

        return Ok(Self {
            name,
            file,
            piece_length,
            pieces_hashes
        });
    }
}

#[derive(Debug)]
pub enum FileMode {
    SingleFile { length: u64 },
    MultipleFiles { files: Vec<FileEntry> }
}

#[derive(Debug)]
pub struct FileEntry {
    pub length: u64,
    pub path: PathBuf
}
