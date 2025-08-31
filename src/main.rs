mod net;
mod util;
mod metrics;
mod config;

use std::convert::TryFrom;
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Duration;
use agentx::encodings;
use agentx::encodings::ID;
use agentx::pdu;
use anyhow::{Result};
use clap::Parser;
use log::{debug, warn};

use config::{load_config};
use metrics::{Metrics};
use net::{rx, tx, txrx};


pub fn run(socket_file: &String, agent_timeout: u64, oid_base: &[u32]) -> Result<()> {
    loop {
        // Connect or reconnect.
        println!("Connecting to snmp daemon on socket {}", socket_file);
        let mut stream = match UnixStream::connect(socket_file) {
            Ok(s) => s,
            Err(e) => {
                println!("Could not connect stream '{}'.  Waiting for a minute.", e);
                thread::sleep(Duration::from_secs(60));
                continue;
            }
        };
        println!("Connected to snmp daemon on socket {}", socket_file);

        if let Err(e) = listen(
            &mut stream, &Metrics::new(),
            Duration::from_secs(agent_timeout), oid_base,
        ) {
            println!("Error while listening: '{}'", e);
        }
        println!("connection broke");
    }
}


fn listen(
    stream: &mut UnixStream,
    metrics: &Metrics,
    agent_timeout: Duration,
    oid_base: &[u32],
) -> Result<()> {
    let agent_id = encodings::ID::try_from(oid_base.to_owned())
        .expect("OID prefix is valid");
    let session_id = create_session(stream, agent_timeout, &agent_id)?;
    register_agent(stream, agent_id, session_id)?;
    // For each request, send a response.
    loop {
        let (typ, bytes) = rx(stream)?;
        debug!("got request '{:?}'", typ);
        let mut resp = match typ {
            pdu::Type::Get => get(&bytes, metrics, oid_base)?,
            pdu::Type::GetNext => get_next(&bytes, metrics)?,
            _ => {
                warn!("listen: header.ty={:?} unknown", typ);
                continue;
            }
        };
        let bytes = resp.to_bytes()?;
        tx(stream, &bytes)?;
    }
}


fn create_session(stream: &mut UnixStream, agent_timeout: Duration, agent_id: &ID) -> Result<u32> {
    let mut open = pdu::Open::new(agent_id.clone(), "AMDGPU");
    open.timeout = agent_timeout;
    let bytes = open.to_bytes().expect("Open PDU can be converted to bytes");
    let resp = txrx(stream, &bytes)?;
    let session_id = resp.header.session_id;
    Ok(session_id)
}


fn register_agent(stream: &mut UnixStream, agent_id: ID, session_id: u32) -> Result<()> {
    let mut register = pdu::Register::new(agent_id);
    register.header.session_id = session_id;
    let bytes = register.to_bytes()
        .expect("Register PDU can be converted to bytes");
    txrx(stream, &bytes)?;
    Ok(())
}


fn get(bytes: &[u8], metrics: &Metrics, oid_base: &[u32]) -> Result<pdu::Response> {
    let pkg = pdu::Get::from_bytes(bytes)?;
    let mut resp = pdu::Response::from_header(&pkg.header);
    let vbl = metrics.get(&pkg.sr, oid_base);
    debug!("get: vbs: {:?}", vbl);
    resp.vb = Some(vbl);
    Ok(resp)
}


fn get_next(bytes: &[u8], metrics: &Metrics) -> Result<pdu::Response> {
    let pkg = pdu::GetNext::from_bytes(bytes)?;
    let mut resp = pdu::Response::from_header(&pkg.header);
    let vbl = metrics.get_next(&pkg.sr);
    debug!("get_next: vbs: {:?}", vbl);
    resp.vb = Some(vbl);
    Ok(resp)
}


#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "~/.config/double-agentx/config.yaml")]
    config_file: String,
    // subcommand: generate_mib_file
}


fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::parse();
    let config = load_config(&args.config_file)?;
    debug!("{:?}", config);
    match run(
        &config.connection.socket,
        config.connection.agent_timeout_seconds,
        &config.oid_base
    ) {
        Ok(()) => println!("Connected successfully!"),
        Err(err) => println!("Failed to connect: {}", err),
    };
    Ok(())
}
