use std::{
    rc::Rc,
    io,
};

use jigsaw_bencode::parser::{self, BencodeParser};

/// Dump error
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    Parse(#[from] parser::Error),
}

pub fn dump_bencode<W: io::Write>(destination: &mut W, bytes: Vec<u8>, debug: bool) -> Result<(), Error> {
    let mut parser = BencodeParser::new(Rc::from(bytes));
    let parsed_file = parser.parse()?;

    if debug {
        writeln!(destination, "{:#?}", parsed_file)?;
    } else {
        writeln!(destination, "{}", parsed_file)?;
    }

    Ok(())
}
