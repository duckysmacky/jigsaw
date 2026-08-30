// using a BTreeMap because it keeps keys in lexicographical order
use std::{
    collections::BTreeMap,
    fmt,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ByteString(pub Vec<u8>);

impl From<&str> for ByteString {
	fn from(value: &str) -> Self {
		ByteString(value.as_bytes().to_vec())
	}
}

impl From<ByteString> for String {
	fn from(value: ByteString) -> Self {
		String::from_utf8_lossy(&value.0).to_string()
	}
}

impl fmt::Debug for ByteString {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = String::from_utf8(self.0.clone())
            .unwrap_or_else(|_| hex::encode(&self.0));

        write!(f, "{}", str)
	}
}

impl fmt::Display for ByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = String::from_utf8(self.0.clone())
            .unwrap_or_else(|_| hex::encode(&self.0));

        write!(f, "{}", str)
    }
}

pub type BencodeDict = BTreeMap<ByteString, BencodeElement>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BencodeElement {
	Int(i64),
	ByteString(ByteString),
	List(Vec<BencodeElement>),
	Dict(BencodeDict),
}
