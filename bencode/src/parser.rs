use std::rc::Rc;

use thiserror::Error;

use crate::{BencodeElement, BencodeDict, BencodeList, ByteString, BencodeNumber, BencodeElementMap, OriginalBytes};

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

pub struct BencodeParser {
    bytes: Rc<[u8]>,
    idx: usize,
}

impl BencodeParser {
    pub fn new(bytes: Rc<[u8]>) -> Self {
        Self {
            bytes,
            idx: 0,
        }
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
            b'i' => { Ok(BencodeElement::Int(self.parse_number()?)) },
            b'0'..=b'9' => { Ok(BencodeElement::String(self.parse_string()?)) }
            _ => { Err(Error::UnexpectedCharacter) }
        }
    }

    fn parse_dict(&mut self) -> Result<BencodeDict, Error> {
        let mut map = BencodeElementMap::new();

        let start = self.idx;
        self.advance()?; // from 'd'
        while self.peek()? != b'e' {
            let key = self.parse_string()?;

            if map.contains_key(&key) {
                return Err(Error::DuplicateKey(key.to_string()));
            }

            map.insert(key, self.parse_elem()?);
        }
        let end = self.idx;
        self.advance()?; // from 'e'

        let original_bytes = OriginalBytes::new(Rc::clone(&self.bytes), start, end);

        Ok(BencodeDict::new(map, original_bytes))
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
            self.advance_by(strlen + 1)?; // ':' + the string
            let str = ByteString::from(self.bytes[self.idx - strlen..self.idx].to_vec());

            return Ok(str);
        }

        Err(Error::InvalidInteger(digits))
    }

    fn parse_number(&mut self) -> Result<BencodeNumber, Error> {
        self.advance()?; // from 'i'
        let mut digits = "".to_string();

        while self.peek()? != b'e' {
            digits.push(self.peek()? as char);
            self.advance()?;
        }
        self.advance()?; // from 'e'

        if let Ok(val) = i64::from_str_radix(&digits, 10) {
            return Ok(BencodeNumber::from(val));
        }

        Err(Error::InvalidInteger(digits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_decoding() {
        let bencoded = b"d3:bar4:spam3:fooi-42e4:listli43ei44ei-73eee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        let mut map = BencodeElementMap::new();
        map.insert("bar".into(), BencodeElement::String("spam".into()));
        map.insert("foo".into(), BencodeElement::Int((-42i64).into()));
        map.insert("list".into(), BencodeElement::List(BencodeList::from(vec![
            BencodeElement::Int((43i64).into()),
            BencodeElement::Int((44i64).into()),
            BencodeElement::Int((-73i64).into())
        ])));

        let original = OriginalBytes::new(Rc::clone(&bytes), 0, bytes.len() - 1);
        let expected = BencodeElement::Dict(BencodeDict::new(map, original));

        assert_eq!(expected, decoder.parse().unwrap());
    }

    #[test]
    fn eof_error() {
        let bencoded = b"d3:bar4:spam3:fooi-42e4:listli43ei44ei-73ee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        assert!(matches!(decoder.parse_dict().expect_err("should have unexpected eof!"), Error::UnexpectedEOF));
    }

    #[test]
    fn zero_length_string() {
        let bencoded = b"d0:i73ee"; // 0 length strings are valid
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        let mut map = BencodeElementMap::new();
        map.insert("".into(), BencodeElement::Int(73i64.into()));

        let original = OriginalBytes::new(Rc::clone(&bytes), 0, bytes.len() - 1);
        let expected = BencodeDict::new(map, original);

        assert_eq!(decoder.parse_dict().expect("zero length string should be valid!"), expected);
    }

    #[test]
    fn negative_string_length() {
        let bencoded = b"d-1:i73ee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        assert!(matches!(decoder.parse_dict().expect_err("negative string length should be invalid!"), Error::NegativeStringLength));
    }

    #[test]
    fn invalid_string_length() {
        let bencoded = b"d3lmao1:testi73ee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        assert!(matches!(decoder.parse_dict().expect_err("string length should be invalid!"), Error::InvalidInteger(int) if int == "3lmao1"));
    }

    #[test]
    fn string_length_too_long() {
        let bencoded = b"d7342:testi43ee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        assert!(matches!(decoder.parse_dict().expect_err("string reading should have gone out of bounds!"), Error::UnexpectedEOF));
    }

    #[test]
    fn unexpected_character() {
        let bencoded = b"d4:spam4:eggs3:foobi43ee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        assert!(matches!(decoder.parse_dict().expect_err("character should be unexpected!"), Error::UnexpectedCharacter));
    }

    #[test]
    fn invalid_integer() {
        let bencoded = b"d4:spami3dsee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        assert!(matches!(decoder.parse_dict().expect_err("integer should be invalid!"), Error::InvalidInteger(int) if int == "3ds"));
    }

    #[test]
    fn duplicate_keys() {
        let bencoded = b"d4:spami3e4:spami4ee";
        let bytes = Rc::from(bencoded.to_vec());
        let mut decoder = BencodeParser::new(Rc::clone(&bytes));

        assert!(matches!(decoder.parse_dict().expect_err("should have caught duplicate keys!"), Error::DuplicateKey(key) if key == "spam"));
    }
}
