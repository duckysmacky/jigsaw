// using a BTreeMap because it keeps keys in lexicographical order
use std::collections::BTreeMap;

use thiserror::Error;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct ByteString(Vec<u8>);

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

impl std::fmt::Debug for ByteString {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_fmt(format_args!("{}", String::from_utf8_lossy(&self.0)))
	}
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum BencodeElem {
	Int(i64),
	ByteString(ByteString),
	List(Vec<BencodeElem>),
	Dict(BTreeMap<ByteString, BencodeElem>),
}

#[derive(Error, Debug)]
enum BencodeError {
	#[error("Unexpected EOF while reading bencoded file.")]
	UnexpectedEOF,
	#[error("Unexpected character while reading bencoded file.")]
	UnexpectedCharacter,
	#[error("Invalid integer '{0}' encountered while reading bencoded file.")]
	InvalidInteger(String),
	#[error("Duplicate dictionary key '{0}' encountered while reading bencoded file.")]
	DuplicateKey(String),
	#[error("ByteString with empty length encountered while reading bencoded file.")]
	EmptyStringLength,
	#[error("ByteString with negative length encountered while reading bencoded file.")]
	NegativeStringLength,
}

struct Bencode<'a> {
	idx: usize,
	file: &'a [u8]
}

impl<'a> Bencode<'a> {
	pub fn new(file: &'a [u8]) -> Bencode<'a> {
		Bencode { idx: 0, file: file }
	}

	fn peek(&self) -> Result<u8, BencodeError> {
		if self.idx >= self.file.len() {
			return Err(BencodeError::UnexpectedEOF);
		}
		Ok(self.file[self.idx])
	}

	fn advance(&mut self) -> Result<(), BencodeError> {
		self.idx += 1;
		if self.idx > self.file.len() {
			return Err(BencodeError::UnexpectedEOF);
		}
		Ok(())
	}

	fn advance_by(&mut self, amt: usize) -> Result<(), BencodeError> {
		self.idx += amt;
		if self.idx > self.file.len() {
			return Err(BencodeError::UnexpectedEOF);
		}
		Ok(())
	}

	fn parse_elem(&mut self) -> Result<BencodeElem, BencodeError> {
		match self.peek()? {
			b'd' => { Ok(BencodeElem::Dict(self.parse_dict()?)) },
			b'l' => { Ok(BencodeElem::List(self.parse_list()?)) },
			b'i' => { Ok(BencodeElem::Int(self.parse_int()?)) },
			b'0'..=b'9' => { Ok(BencodeElem::ByteString(self.parse_string()?)) }
			// TODO: error out
			_ => { Err(BencodeError::UnexpectedCharacter) }
		}
	}

	pub fn parse_dict(&mut self) -> Result<BTreeMap<ByteString, BencodeElem>, BencodeError> {
		let mut res: BTreeMap<ByteString, BencodeElem> = BTreeMap::new();

		self.advance()?; // from 'd'
		while self.peek()? != b'e' {
			let key: ByteString = self.parse_string()?;
			if res.contains_key(&key) {
				return Err(BencodeError::DuplicateKey(key.into()));
			}
			res.insert(key, self.parse_elem()?);
		}
		self.advance()?; // from 'e'

		Ok(res)
	}

	fn parse_list(&mut self) -> Result<Vec<BencodeElem>, BencodeError> {
		self.advance()?; // from 'l'
		let mut res: Vec<BencodeElem> = Vec::new();
		while self.peek()? != b'e' {
			res.push(self.parse_elem()?);
		}
		self.advance()?; // from 'e'

		Ok(res)
	}

	fn parse_string(&mut self) -> Result<ByteString, BencodeError> {
		let mut digits: String = "".to_string();
		while self.peek()? != b':' {
			digits.push(self.peek()? as char);
			self.advance()?;
		}

		if digits.len() == 0 {
			return Err(BencodeError::EmptyStringLength);
		}

		// unwrap safety: we checked for empty string
		if digits.chars().nth(0).unwrap() == '-' {
			return Err(BencodeError::NegativeStringLength);
		}

		if let Ok(strlen) = usize::from_str_radix(&digits, 10) {
			self.advance()?; // from ':'
			let res: ByteString = ByteString(self.file[self.idx..self.idx + strlen].to_vec());
			self.advance_by(strlen)?;

			return Ok(res);
		}

		Err(BencodeError::InvalidInteger(digits))
	}

	fn parse_int(&mut self) -> Result<i64, BencodeError> {
		self.advance()?; // from 'i'
		let mut digits: String = "".to_string();
		while self.peek()? != b'e' {
			digits.push(self.peek()? as char);
			self.advance()?;
		}
		self.advance()?; // from 'e'

		if let Ok(val) = i64::from_str_radix(&digits, 10) {
			return Ok(val);
		}

		Err(BencodeError::InvalidInteger(digits))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// TODO: add more tests

	#[test]
	fn correct_decoding() {
		let bencoded = b"d3:bar4:spam3:fooi-42e4:listli43ei44ei-73eee";
		let mut decoder: Bencode = Bencode::new(bencoded);

		let mut expected: BTreeMap<ByteString, BencodeElem> = BTreeMap::new();
		expected.insert("bar".into(), BencodeElem::ByteString("spam".into()));
		expected.insert("foo".into(), BencodeElem::Int(-42));
		expected.insert("list".into(), BencodeElem::List(vec![BencodeElem::Int(43), BencodeElem::Int(44), BencodeElem::Int(-73)]));

		assert_eq!(expected, decoder.parse_dict().unwrap());
	}
}