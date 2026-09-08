use std::{
    rc::Rc,
};

use tokio::io::{self, AsyncWrite, AsyncWriteExt};

use jigsaw_bencode::parser::{self, BencodeParser};

/// Dump error
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    Parse(#[from] parser::Error),
}

pub async fn dump_bencode<W>(destination: &mut W, bytes: Vec<u8>, debug: bool) -> Result<(), Error>
where
    W: AsyncWrite + Unpin + ?Sized,
{
    let mut parser = BencodeParser::new(Rc::from(bytes));
    let parsed_file = parser.parse()?;

    let output = if debug {
        format!("{:#?}\n", parsed_file)
    } else {
        format!("{}\n", parsed_file)
    };

    destination.write_all(output.as_bytes()).await?;

    Ok(())
}
