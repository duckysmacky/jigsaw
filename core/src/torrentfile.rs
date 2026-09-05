use std::path::PathBuf;

use sha1::{Digest, Sha1};
use thiserror::Error;

use jigsaw_bencode::{BencodeElement, ByteString};

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
    /// SHA1 of the bencoded `Info` dict
    pub info_hash: [u8; 20],

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
        let announce = bencode_get!(dict: required "announce", String => |x: &ByteString| x.to_string())?;
        let (info, info_hash) = bencode_get!(dict: required "info", Dict => errors |info_dict: &BencodeDict| {
            let info = Info::from_bencoded(info_dict)?;
            let info_hash = hash_info_dict(info_dict);
            Ok((info, info_hash))
        })?;
        let comment = bencode_get!(dict: optional "comment", String => |x: &ByteString| x.to_string())?;
        let created_by = bencode_get!(dict: optional "created by", String => |x: &ByteString| x.to_string())?;
        let creation_date = bencode_get!(dict: optional "creation date", Number => |x: &i64| *x as u64)?;

        Ok(Self {
            announce,
            info,
            info_hash,
            comment,
            created_by,
            creation_date
        })
    }
}

fn hash_info_dict(info_dict: &BencodeDict) -> [u8; 20] {
    let bytes = info_dict.original_bytes();

    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let result = hasher.finalize();

    result.into()
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

        let name = bencode_get!(info_dict: required "name", String => |x: &ByteString| x.to_string())?;

        let file = if info_dict.contains_key(&"files".into()) {
            let files = match &info_dict[&"files".into()] {
                BencodeElement::List(files_list) => {
                    let mut file_entries: Vec<FileEntry> = Vec::new();
                    for entry_elem in files_list {
                        match entry_elem {
                            BencodeElement::Dict(entry_dict) => {
                                let length = bencode_get!(entry_dict: required "length", Number => |len: &i64| *len as u64)?;

                                let path = match entry_dict.get(&"path".into()) {
                                    Some(BencodeElement::List(path_list)) => {
                                        let mut path_parts = PathBuf::new();
                                        for path_part in path_list {
                                            match path_part {
                                                BencodeElement::String(part) => path_parts.push(part.to_string()),
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
            let length = bencode_get!(info_dict: required "length", Number => errors |len: &i64| {
                if *len < 0 { return Err(StructureError::NegativeLength) }
                Ok(*len as u64)
            })?;

            FileMode::SingleFile { length }
        };

        let piece_length = bencode_get!(info_dict: required "piece length", Number => errors |len: &i64| {
            if *len < 0 { return Err(StructureError::NegativeLength) }
            Ok(*len as u32)
        })?;

        let pieces_hashes = bencode_get!(info_dict: required "pieces", String => errors |bytes: &ByteString| {
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

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use jigsaw_bencode::BencodeParser;

    use super::*;

    fn bstring(s: &str) -> String {
        format!("{}:{}", s.len(), s)
    }

    fn bint(n: i64) -> String {
        format!("i{}e", n)
    }

    fn key_value(key: &str, value: &str) -> String {
        format!("{}{}", bstring(key), value)
    }

    fn single_file_info() -> String {
        let piece_hash = "01234567890123456789"; // 20 bytes, one fake piece
        format!(
            "d{}{}{}{}e",
            key_value("length", &bint(12345)),
            key_value("name", &bstring("test.txt")),
            key_value("piece length", &bint(16384)),
            key_value("pieces", &bstring(piece_hash)),
        )
    }

    fn multi_file_info() -> String {
        let piece_hash = "01234567890123456789".repeat(2); // 40 bytes, two fake pieces
        let file_a = format!(
            "d{}{}e",
            key_value("length", &bint(100)),
            key_value("path", &format!("l{}e", bstring("dir/file_a.txt"))),
        );
        let file_b = format!(
            "d{}{}e",
            key_value("length", &bint(200)),
            key_value("path", &format!("l{}{}e", bstring("dir"), bstring("file_b.txt"))),
        );
        format!(
            "d{}{}{}{}e",
            key_value("files", &format!("l{}{}e", file_a, file_b)),
            key_value("name", &bstring("test_dir")),
            key_value("piece length", &bint(16384)),
            key_value("pieces", &bstring(&piece_hash)),
        )
    }

    fn parse_dict(bencoded: &str) -> BencodeDict {
        let bytes: Rc<[u8]> = Rc::from(bencoded.as_bytes().to_vec());
        let mut parser = BencodeParser::new(bytes);

        match parser.parse().expect("input should be valid bencode") {
            BencodeElement::Dict(dict) => dict,
            _ => panic!("top-level bencode element should be a dict"),
        }
    }

    fn expected_hash(info: &str) -> [u8; 20] {
        use sha1::{Digest, Sha1};

        let mut hasher = Sha1::new();
        hasher.update(info.as_bytes());
        hasher.finalize().into()
    }

    #[test]
    fn parses_single_file_torrent() {
        let info = single_file_info();
        let torrent = format!(
            "d{}{}{}{}{}e",
            key_value("announce", &bstring("http://tracker.example.com/announce")),
            key_value("comment", &bstring("test comment")),
            key_value("created by", &bstring("jigsaw-test")),
            key_value("creation date", &bint(1700000000)),
            key_value("info", &info),
        );

        let dict = parse_dict(&torrent);
        let torrent_file = TorrentFile::from_bencoded(dict).expect("structure should be valid");

        assert_eq!(torrent_file.announce, "http://tracker.example.com/announce");
        assert_eq!(torrent_file.comment.as_deref(), Some("test comment"));
        assert_eq!(torrent_file.created_by.as_deref(), Some("jigsaw-test"));
        assert_eq!(torrent_file.creation_date, Some(1700000000));

        assert_eq!(torrent_file.info.name, "test.txt");
        assert_eq!(torrent_file.info.piece_length, 16384);
        assert_eq!(torrent_file.info.pieces_hashes.len(), 1);
        assert!(matches!(torrent_file.info.file, FileMode::SingleFile { length: 12345 }));

        assert_eq!(torrent_file.info_hash, expected_hash(&info));
    }

    #[test]
    fn parses_multi_file_torrent() {
        let info = multi_file_info();
        let torrent = format!("d{}{}e", key_value("announce", &bstring("http://tracker.example.com/announce")), key_value("info", &info));

        let dict = parse_dict(&torrent);
        let torrent_file = TorrentFile::from_bencoded(dict).expect("structure should be valid");

        assert_eq!(torrent_file.comment, None);
        assert_eq!(torrent_file.created_by, None);
        assert_eq!(torrent_file.creation_date, None);

        assert_eq!(torrent_file.info.name, "test_dir");
        assert_eq!(torrent_file.info.pieces_hashes.len(), 2);

        match torrent_file.info.file {
            FileMode::MultipleFiles { files } => {
                assert_eq!(files.len(), 2);
                assert_eq!(files[0].length, 100);
                assert_eq!(files[0].path, PathBuf::from("dir/file_a.txt"));
                assert_eq!(files[1].length, 200);
                assert_eq!(files[1].path, PathBuf::from("dir/file_b.txt"));
            }
            _ => panic!("expected multiple files"),
        }

        assert_eq!(torrent_file.info_hash, expected_hash(&info));
    }

    #[test]
    fn fails_when_announce_is_missing() {
        let info = single_file_info();
        let torrent = format!("d{}e", key_value("info", &info));

        let dict = parse_dict(&torrent);
        let err = TorrentFile::from_bencoded(dict).unwrap_err();

        assert!(matches!(err, StructureError::RequiredKeyMissing(key) if key == "announce"));
    }

    #[test]
    fn fails_when_pieces_length_is_not_divisible_by_20() {
        let info = format!(
            "d{}{}{}{}e",
            key_value("length", &bint(1)),
            key_value("name", &bstring("test.txt")),
            key_value("piece length", &bint(16384)),
            key_value("pieces", &bstring("short")),
        );
        let torrent = format!("d{}{}e", key_value("announce", &bstring("http://tracker.example.com/announce")), key_value("info", &info));

        let dict = parse_dict(&torrent);
        let err = TorrentFile::from_bencoded(dict).unwrap_err();

        assert!(matches!(err, StructureError::PiecesBytesLengthError));
    }
}
