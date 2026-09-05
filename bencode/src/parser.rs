use thiserror::Error;

use crate::{BencodeElement, BencodeDict, BencodeList, ByteString};

#[derive(Error, Debug)]
pub enum Error {
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

pub struct BencodeParser<'a> {
    idx: usize,
    bytes: &'a [u8]
}

impl<'a> BencodeParser<'a> {
    pub fn new(bytes: &'a [u8]) -> BencodeParser<'a> {
        BencodeParser { idx: 0, bytes }
    }

    pub fn parse(&mut self) -> Result<BencodeElement, Error> {
        // TODO: expect any element when parsing, not just dict
        let dict = self.parse_dict()?;
        Ok(BencodeElement::Dict(dict))
    }

    fn peek(&self) -> Result<u8, Error> {
        if self.idx >= self.bytes.len() {
            return Err(Error::UnexpectedEOF);
        }
        Ok(self.bytes[self.idx])
    }

    fn advance(&mut self) -> Result<(), Error> {
        self.idx += 1;
        if self.idx > self.bytes.len() {
            return Err(Error::UnexpectedEOF);
        }
        Ok(())
    }

    fn advance_by(&mut self, amt: usize) -> Result<(), Error> {
        self.idx += amt;
        if self.idx > self.bytes.len() {
            return Err(Error::UnexpectedEOF);
        }
        Ok(())
    }

    fn parse_elem(&mut self) -> Result<BencodeElement, Error> {
        match self.peek()? {
            b'd' => { Ok(BencodeElement::Dict(self.parse_dict()?)) },
            b'l' => { Ok(BencodeElement::List(self.parse_list()?)) },
            b'i' => { Ok(BencodeElement::Int(self.parse_int()?)) },
            b'0'..=b'9' => { Ok(BencodeElement::ByteString(self.parse_string()?)) }
            // TODO: error out
            _ => { Err(Error::UnexpectedCharacter) }
        }
    }

    fn parse_dict(&mut self) -> Result<BencodeDict, Error> {
        let mut dict = BencodeDict::new();

        self.advance()?; // from 'd'
        while self.peek()? != b'e' {
            let key = self.parse_string()?;

            if dict.contains_key(&key) {
                return Err(Error::DuplicateKey(key.to_string()));
            }

            dict.insert(key, self.parse_elem()?);
        }
        self.advance()?; // from 'e'

        Ok(dict)
    }

    fn parse_list(&mut self) -> Result<BencodeList, Error> {
        self.advance()?; // from 'l'
        let mut list = BencodeList::new();

        while self.peek()? != b'e' {
            list.push(self.parse_elem()?);
        }
        self.advance()?; // from 'e'

        Ok(list)
    }

    fn parse_string(&mut self) -> Result<ByteString, Error> {
        let mut digits = "".to_string();
        while self.peek()? != b':' {
            digits.push(self.peek()? as char);
            self.advance()?;
        }

        if digits.len() == 0 {
            return Err(Error::EmptyStringLength);
        }

        // unwrap safety: we checked for empty string
        if digits.chars().nth(0).unwrap() == '-' {
            return Err(Error::NegativeStringLength);
        }

        if let Ok(strlen) = usize::from_str_radix(&digits, 10) {
            self.advance()?; // from ':'
            let str = ByteString::from(self.bytes[self.idx..self.idx + strlen].to_vec());
            self.advance_by(strlen)?;

            return Ok(str);
        }

        Err(Error::InvalidInteger(digits))
    }

    fn parse_int(&mut self) -> Result<i64, Error> {
        self.advance()?; // from 'i'
        let mut digits = "".to_string();

        while self.peek()? != b'e' {
            digits.push(self.peek()? as char);
            self.advance()?;
        }
        self.advance()?; // from 'e'

        if let Ok(val) = i64::from_str_radix(&digits, 10) {
            return Ok(val);
        }

        Err(Error::InvalidInteger(digits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: add more tests

    #[test]
    fn correct_decoding() {
        let bencoded = b"d3:bar4:spam3:fooi-42e4:listli43ei44ei-73eee";
        let mut decoder = BencodeParser::new(bencoded);

        let mut dict = BencodeDict::new();
        dict.insert("bar".into(), BencodeElement::ByteString("spam".into()));
        dict.insert("foo".into(), BencodeElement::Int(-42));
        dict.insert("list".into(), BencodeElement::List(BencodeList::from(vec![BencodeElement::Int(43), BencodeElement::Int(44), BencodeElement::Int(-73)])));
        let expected = BencodeElement::Dict(dict);

        assert_eq!(expected, decoder.parse().unwrap());
    }
}
