use std::path::PathBuf;

use jigsaw_bencode::{BencodeElement, ByteString as ByteStringType};
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

/// Extract and parse bencode dictionary values with automatic error handling.
///
/// This macro eliminates boilerplate when extracting typed values from bencoded dictionaries.
/// It handles type checking and provides consistent error messages for missing/wrong-type fields.
///
/// Four forms:
/// - `required $key, Type => $parse_fn`: Extract required field and parse with non-fallible function
/// - `optional $key, Type => $parse_fn`: Extract optional field, returns `Option<T>` with non-fallible parse
/// - `required $key, Type => errors $parse_fn`: Extract required field with fallible parser returning `Result<T, StructureError>`
/// - `optional $key, Type => errors $parse_fn`: Extract optional field with fallible parser, returns `Option<T>`
///
/// # Example
/// ```ignore
/// // Non-fallible parsing (simple value transformation)
/// let name = bencode_get!(dict: required "name", ByteString => |x| x.to_string())?;
///
/// // Fallible parsing (with validation)
/// let length = bencode_get!(dict: required "length", Int => errors |len: &i64| {
///     if *len < 0 { return Err(StructureError::NegativeLength) }
///     Ok(*len as u64)
/// })?;
/// ```
macro_rules! bencode_get {
    ($dict:tt : required $key:expr, $element_type:ident => $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => Ok($parse_fn(val)),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Err(StructureError::RequiredKeyMissing($key.to_string())),
        }
    }};
    ($dict:tt : optional $key:expr, $element_type:ident => $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => Ok(Some($parse_fn(val))),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Ok(None),
        }
    }};
    ($dict:tt : required $key:expr, $element_type:ident => errors $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => $parse_fn(val),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Err(StructureError::RequiredKeyMissing($key.to_string())),
        }
    }};
    ($dict:tt : optional $key:expr, $element_type:ident => errors $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => $parse_fn(val).map(Some),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Ok(None),
        }
    }};
}

impl TorrentFile {
    pub fn from_bencoded(dict: BencodeDict) -> Result<Self, StructureError> {
        let announce = bencode_get!(dict: required "announce", ByteString => |x: &ByteStringType| x.to_string())?;
        let info = bencode_get!(dict: required "info", Dict => errors |x: &BencodeDict| Info::from_bencoded(x))?;
        let comment = bencode_get!(dict: optional "comment", ByteString => |x: &ByteStringType| x.to_string())?;
        let created_by = bencode_get!(dict: optional "created by", ByteString => |x: &ByteStringType| x.to_string())?;
        let creation_date = bencode_get!(dict: optional "creation date", Int => |x: &i64| *x as u64)?;

        Ok(Self {
            announce,
            info,
            comment,
            created_by,
            creation_date
        })
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

        let name = bencode_get!(info_dict: required "name", ByteString => |x: &ByteStringType| x.to_string())?;

        let file = if info_dict.contains_key(&"files".into()) {
            let files = match &info_dict[&"files".into()] {
                BencodeElement::List(files_list) => {
                    let mut file_entries: Vec<FileEntry> = Vec::new();
                    for entry_elem in files_list {
                        match entry_elem {
                            BencodeElement::Dict(entry_dict) => {
                                let length = bencode_get!(entry_dict: required "length", Int => |len: &i64| *len as u64)?;

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
            let length = bencode_get!(info_dict: required "length", Int => errors |len: &i64| {
                if *len < 0 { return Err(StructureError::NegativeLength) }
                Ok(*len as u64)
            })?;

            FileMode::SingleFile { length }
        };

        let piece_length = bencode_get!(info_dict: required "piece length", Int => errors |len: &i64| {
            if *len < 0 { return Err(StructureError::NegativeLength) }
            Ok(*len as u32)
        })?;

        let pieces_hashes = bencode_get!(info_dict: required "pieces", ByteString => errors |bytes: &ByteStringType| {
            if bytes.len() % 20 != 0 {
                return Err(StructureError::PiecesBytesLengthError);
            }
            // unwrap will not fail, the chunk size is guaranteed.
            Ok(bytes.bytes()
                .chunks_exact(20)
                .map(|ch| ch.try_into().unwrap())
                .collect::<Vec<[u8; 20]>>())
        })?;

        Ok(Self {
            name,
            file,
            piece_length,
            pieces_hashes
        })
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
