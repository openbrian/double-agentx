use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use agentx::pdu;
use anyhow::Result;


pub fn txrx(stream: &mut UnixStream, bytes: &[u8]) -> Result<pdu::Response> {
    tx(stream, bytes)?;
    let (_, buf) = rx(stream)?;
    Ok(pdu::Response::from_bytes(&buf)?)
}


pub fn tx(stream: &mut UnixStream, bytes: &[u8]) -> Result<()> {
    stream.write_all(bytes)?;
    Ok(())
}


pub fn rx(stream: &mut UnixStream) -> Result<(pdu::Type, Vec<u8>)> {
    let mut buf = vec![0u8; 20];

    stream.read_exact(&mut buf)?;
    let header = pdu::Header::from_bytes(&buf)?;
    buf.resize(20 + header.payload_length as usize, 0);
    stream.read_exact(&mut buf[20..])?;
    Ok((header.ty, buf))
}
