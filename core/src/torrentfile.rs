use jigsaw_bencode::BencodeElement;
use thiserror::Error;

use crate::bencode::BencodeDict;

// TODO: bittorrent v2 does things differently

#[derive(Error, Debug)]
pub enum StructureError {
    #[error("Field '{0}' is of wrong type.")]
    WrongType(String),
    #[error("Dictionary '{0}' is missing required key '{1}'.")]
    RequiredKeyMissing(String, String),
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

    // these aren't required keys, maybe put them in an Option
    pub comment: String,
    pub created_by: String,
    pub creation_date: u64,
}

impl TorrentFile {
    pub fn new() -> TorrentFile {
        TorrentFile {
            announce: "".to_string(),
            info: Info::new(),
            comment: "".to_string(),
            created_by: "".to_string(),
            creation_date: 0,
        }
    }

    pub fn from_bencoded(dict: &BencodeDict) -> Result<TorrentFile, StructureError> {
        let mut file = TorrentFile::new();

        if let BencodeElement::ByteString(announce) = &dict[&"announce".into()] {
            file.announce = announce.to_string();
        } else {
            return Err(StructureError::WrongType("announce".to_string()));
        }

        if let BencodeElement::Dict(info_dict) = &dict[&"info".into()] {
            file.info = Info::from_bencoded(info_dict)?;
        } else {
            return Err(StructureError::WrongType("info".to_string()));
        }

        // won't error out if wrong type, will just ignore since these aren't required keys
        if dict.contains_key(&"comment".into()) && let BencodeElement::ByteString(comment) = &dict[&"comment".into()] {
            file.comment = comment.to_string();
        }
        if dict.contains_key(&"created by".into()) && let BencodeElement::ByteString(created_by) = &dict[&"created by".into()] {
            file.created_by = created_by.to_string();
        }
        if dict.contains_key(&"creation date".into()) && let BencodeElement::Int(creation_date) = dict[&"creation date".into()] {
            file.creation_date = creation_date as u64;
        }

        return Ok(file);
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
    pub fn new() -> Info {
        Info {
            name: "".to_string(),
            file: FileMode::SingleFile { length: 0 },
            piece_length: 0,
            pieces_hashes: Vec::new()
        }
    }

    pub fn from_bencoded(info_dict: &BencodeDict) -> Result<Info, StructureError> {
        let mut info = Info::new();

        // required keys
        for key in ["name", "pieces", "piece length"] {
            if !info_dict.contains_key(&key.into()) {
                return Err(StructureError::RequiredKeyMissing("info".to_string(), key.to_string()));
            }
        }
        // either one or the other should be present
        if !info_dict.contains_key(&"files".into()) && !info_dict.contains_key(&"length".into()) {
            return Err(StructureError::RequiredKeyMissing("info".to_string(), "files/length".to_string()));
        }

        if let BencodeElement::ByteString(name) = &info_dict[&"name".into()] {
            info.name = name.to_string();
        } else {
            return Err(StructureError::WrongType("name".to_string()))
        }

        // yucky disgusting im sorry
        if info_dict.contains_key(&"files".into()) {
            if let BencodeElement::List(files_list) = &info_dict[&"files".into()] {
                let mut entries: Vec<FileEntry> = Vec::new();
                for entry_elem in files_list {
                    if let BencodeElement::Dict(entry_dict) = entry_elem {
                        if !entry_dict.contains_key(&"length".into()) {
                            return Err(StructureError::RequiredKeyMissing("(file entry)".to_string(), "length".to_string()));
                        }
                        if !entry_dict.contains_key(&"path".into()) {
                            return Err(StructureError::RequiredKeyMissing("(file entry)".to_string(), "path".to_string()));
                        }

                        let mut entry: FileEntry = FileEntry::new();

                        if let BencodeElement::Int(len) = entry_dict[&"length".into()] {
                            entry.length = len as u64;
                        } else {
                            return Err(StructureError::WrongType("length".to_string()));
                        }

                        if let BencodeElement::List(path_list) = &entry_dict[&"path".into()] {
                            for elem in path_list {
                                if let BencodeElement::ByteString(path_part) = elem {
                                    entry.path.push(path_part.to_string());
                                } else {
                                    return Err(StructureError::WrongType("(path part)".to_string()));
                                }
                            }
                        } else {
                            return Err(StructureError::WrongType("path".to_string()));
                        }

                        entries.push(entry);
                    } else {
                        return Err(StructureError::WrongType("(file entry)".to_string()));
                    }
                }
                info.file = FileMode::MultipleFiles { files: entries }
            } else {
                return Err(StructureError::WrongType("files".to_string()))
            }
        } else {
            if let BencodeElement::Int(length) = info_dict[&"length".into()] {
                if length < 0 {
                    return Err(StructureError::NegativeLength);
                }
                info.file = FileMode::SingleFile { length: length as u64 };
            } else {
                return Err(StructureError::WrongType("length".to_string()));
            }
        }

        if let BencodeElement::Int(piece_length) = info_dict[&"piece length".into()] {
            if piece_length < 0 {
                return Err(StructureError::NegativeLength);
            }
            info.piece_length = piece_length as u32;
        } else {
            return Err(StructureError::WrongType("piece length".to_string()));
        }

        if let BencodeElement::ByteString(bytes) = &info_dict[&"pieces".into()] {
            if bytes.len() % 20 != 0 {
                return Err(StructureError::PiecesBytesLengthError);
            }
            // unwrap will not fail, the chunk size is guaranteed.
            info.pieces_hashes = bytes.inner()
                .chunks_exact(20)
                .map(|ch| ch.try_into().unwrap())
                .collect();
        } else {
            return Err(StructureError::WrongType("pieces".to_string()))
        }

        return Ok(info);
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
    pub path: Vec<String>,
}

impl FileEntry {
    pub fn new() -> FileEntry {
        FileEntry { length: 0, path: Vec::new() }
    }
}
