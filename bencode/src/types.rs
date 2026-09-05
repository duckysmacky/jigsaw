// using a BTreeMap because it keeps keys in lexicographical order
use std::{
    ops::{Deref, DerefMut},
    collections::BTreeMap,
    rc::Rc,
    fmt,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct ByteString {
    inner: Vec<u8>,
}

impl ByteString {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.inner
    }

    pub fn display(&self) -> String {
        format!("{}:{}", self.len(), self)
    }
}

impl From<String> for ByteString {
    fn from(value: String) -> Self {
        Self {
            inner: value.into_bytes(),
        }
    }
}

impl From<&str> for ByteString {
    fn from(value: &str) -> Self {
        Self {
            inner: value.as_bytes().to_vec(),
        }
    }
}

impl From<Vec<u8>> for ByteString {
    fn from(value: Vec<u8>) -> Self {
        Self {
            inner: value,
        }
    }
}

impl fmt::Debug for ByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = String::from_utf8(self.inner.clone())
            .unwrap_or_else(|_| hex::encode(&self.inner));

        write!(f, "{}", str)
    }
}

impl fmt::Display for ByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = String::from_utf8(self.inner.clone())
            .unwrap_or_else(|_| hex::encode(&self.inner));

        write!(f, "{}", str)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct BencodeNumber {
    inner: i64,
}

impl BencodeNumber {
    pub fn display(&self) -> String {
        format!("i{}e", self)
    }
}

impl From<i64> for BencodeNumber {
    fn from(value: i64) -> Self {
        Self { inner: value }
    }
}

impl Deref for BencodeNumber {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Display for BencodeNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct BencodeList {
    inner: Vec<BencodeElement>,
}

impl BencodeList {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
        }
    }
}

impl From<Vec<BencodeElement>> for BencodeList {
    fn from(value: Vec<BencodeElement>) -> Self {
        Self {
            inner: value
        }
    }
}

impl Deref for BencodeList {
    type Target = Vec<BencodeElement>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for BencodeList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl IntoIterator for BencodeList {
    type Item = BencodeElement;
    type IntoIter = std::vec::IntoIter<BencodeElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a BencodeList {
    type Item = &'a BencodeElement;
    type IntoIter = std::slice::Iter<'a, BencodeElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl fmt::Debug for BencodeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl BencodeList {
    fn fmt_indent(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "l\n")?;

        for val in self.inner.iter() {
            write!(f, "{}", "\t".repeat(level + 1))?;
            val.fmt_indent(f, level + 1)?;
            write!(f, "\n")?;
        }

        write!(f, "{}e", "\t".repeat(level))
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl fmt::Display for BencodeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_indent(f, 0)
    }
}

pub type BencodeElementMap = BTreeMap<ByteString, BencodeElement>;

/// Struct used to hold a reference to a byte array and the inclusive
/// indexes `start` and `end` of where the original bytes are located
/// in the array
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct OriginalBytes {
    bytes: Rc<[u8]>,
    start: usize,
    end: usize,
}

impl OriginalBytes {
    pub fn new(bytes: Rc<[u8]>, start: usize, end: usize) -> Self {
        Self {
            bytes,
            start,
            end,
        }
    }

    pub fn get(&self) -> &[u8] {
        &self.bytes[self.start..self.end + 1]
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct BencodeDict {
    inner: BencodeElementMap,
    original_bytes: OriginalBytes,
}

impl BencodeDict {
    pub fn new(inner: BencodeElementMap, original_bytes: OriginalBytes) -> Self {
        Self {
            inner,
            original_bytes,
        }
    }

    pub fn original_bytes(&self) -> &[u8] {
        self.original_bytes.get()
    }
}

impl Deref for BencodeDict {
    type Target = BencodeElementMap;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for BencodeDict {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl fmt::Debug for BencodeDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl BencodeDict {
    fn fmt_indent(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "d\n")?;

        for (key, value) in self.iter() {
            write!(f, "{}{}\n", "\t".repeat(level + 1), key.display())?;
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
    Number(BencodeNumber),
    String(ByteString),
    List(BencodeList),
    Dict(BencodeDict),
}

impl BencodeElement {
    fn fmt_indent(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        match self {
            BencodeElement::Number(val) => write!(f, "{}", val.display()),
            BencodeElement::String(val) => write!(f, "{}", val.display()),
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
