// using a BTreeMap because it keeps keys in lexicographical order
use std::{
    ops::{Deref, DerefMut},
    collections::BTreeMap,
    fmt, 
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct ByteString(Vec<u8>);

impl ByteString {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<&str> for ByteString {
	fn from(value: &str) -> Self {
		ByteString(value.as_bytes().to_vec())
	}
}

impl From<Vec<u8>> for ByteString {
    fn from(value: Vec<u8>) -> Self {
        ByteString(value)
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

        write!(f, "{}:{}", self.0.len(), str)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct BencodeList(Vec<BencodeElement>);

impl BencodeList {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl From<Vec<BencodeElement>> for BencodeList {
    fn from(value: Vec<BencodeElement>) -> Self {
        BencodeList(value)
    }
}

impl Deref for BencodeList {
    type Target = Vec<BencodeElement>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BencodeList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Debug for BencodeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl BencodeList {
    fn fmt_indent(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "l\n")?;

        for val in self.0.iter() {
            write!(f, "{}", "\t".repeat(level + 1))?;
            val.fmt_indent(f, level + 1)?;
            write!(f, "\n")?;
        }

        write!(f, "{}e", "\t".repeat(level))
    }
}

impl fmt::Display for BencodeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_indent(f, 0)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct BencodeDict(BTreeMap<ByteString, BencodeElement>);

impl BencodeDict {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl Deref for BencodeDict {
    type Target = BTreeMap<ByteString, BencodeElement>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BencodeDict {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Debug for BencodeDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl BencodeDict {
    fn fmt_indent(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "d\n")?;

        for (key, value) in self.iter() {
            write!(f, "{}{}\n", "\t".repeat(level + 1), key)?;
            write!(f, "\t{}", "\t".repeat(level + 1))?;
            value.fmt_indent(f, level + 1)?;
            write!(f, "\n")?;
        }

        write!(f, "{}e", "\t".repeat(level))
    }
}

impl fmt::Display for BencodeDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_indent(f, 0)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub enum BencodeElement {
	Int(i64),
	ByteString(ByteString),
	List(BencodeList),
	Dict(BencodeDict),
}

impl BencodeElement {
    fn fmt_indent(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        match self {
            BencodeElement::Int(val) => write!(f, "i{}e", val),
            BencodeElement::ByteString(val) => write!(f, "{}", val),
            BencodeElement::List(val) => val.fmt_indent(f, level),
            BencodeElement::Dict(val) => val.fmt_indent(f, level),
        }
    }
}

impl fmt::Display for BencodeElement {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_indent(f, 0)
    }
}
