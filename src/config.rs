use anyhow::{Context, Result};
use log::{debug};
use std::fs::read_to_string;
use serde::Deserialize;


#[derive(Deserialize, Debug)]
pub struct Config {
    pub metrics: Vec<Metric>,
    pub connection: Connection,
    pub oid_base: Vec<u32>,
}


#[derive(Clone, Deserialize, Debug)]
pub struct Metric {
    pub command: String,
    pub name: String,
    pub relative_oid: Vec<u32>,
}


#[derive(Deserialize, Debug)]
pub struct Connection {
    pub socket: String,
    pub agent_timeout_seconds: u64,
}


pub fn load_config(path: &String) -> Result<Config> {
    debug!("Loading config from {}", path);
    let home = std::env::var("HOME").context("HOME")?;
    let file_name = path.replace('~', &home);
    let config: Config = serde_yaml::from_str(
        &read_to_string(&file_name).context(file_name)?
    )?;
    Ok(config)
}
