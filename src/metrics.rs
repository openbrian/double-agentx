use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Bound;
use std::process::Command;
use agentx::encodings;
use anyhow::{anyhow, Result};
use log::{debug, info};
use serde_json::Value;
use crate::config::{Metric};
use crate::entry::{Entry, RecursiveIterator};
use crate::util::{as_vec};

extern crate jsonpath_lib as jsonpath;


pub(crate) struct Metrics {
    // mib: BTreeMap<encodings::ID, encodings::Value>,
    mib: RefCell<BTreeMap<Vec<u32>, encodings::Value>>,
    // TODO: Consider not having BTree of oids, but the oid tree itself.  It
    // will be harder to get ranges.
    // TODO: Consider prepopulating this with the number of instances needed.
    // There's no need to recreate oids.
    // oid_base: Vec<u32>,
    config: Metric,
}


impl Metrics {
    pub(crate) fn new(oid_base: &Vec<u32>, config: &Metric) -> Self {
        debug!("Creating new Metrics");
        debug!("oid_base: {:?}", oid_base);
        debug!("config: {:?}", config);
        Self {
            mib: RefCell::new(BTreeMap::new()),
            // oid_base: [oid_base.to_owned(), config.relative_oid.clone()].concat(),
            config: config.clone(),
        }
    }


    fn generate_mib(&self) -> Result<()> {
        // Currently we get all info.  Not sure if getting any specific data
        // is costly, but if so, we can break this up into showfan, showpower,
        // showtemp, etc.

        let stdout = run_command(&self.config.command)?;
        let data= serde_json::from_str(&stdout)
            .expect("failed to parse json");
        let mut selector = jsonpath::selector(&data);

        let mut tree = self.mib.borrow_mut();
        tree.clear();

        // Walk through config mibs.
        let mut iterator = RecursiveIterator::new(&self.config.mibs);
        while let Some(obj) = iterator.next() {
            let value: Option<encodings::Value> = match detect_handler(&obj) {
                Handler::Literal(literal) => {
                    Some(encodings::Value::OctetString(
                        encodings::OctetString(literal)
                    ))
                },
                Handler::Json(path) => {
                    debug!("path: {:?}", path);
                    let values = selector(&path)?;
                    if let Some(value) = values.first() {
                        match value {
                            Value::String(s) => {
                                let mut val = MibType::String(s.clone());
                                if let Some(converter) = &obj.convert {
                                    val = convert(val, converter)?;
                                }
                                match val {
                                    MibType::Integer(i) => {
                                        Some(encodings::Value::Integer(i as i32))
                                    },
                                    MibType::String(s) => {
                                        Some(encodings::Value::OctetString(
                                            encodings::OctetString(s)
                                        ))
                                    },
                                    _ => {
                                        return Err(anyhow!("No data type handler for {:?}", value));
                                    }
                                }
                            },
                            _ => {
                                return Err(anyhow!("No data type handler for {:?}", value));
                            }
                        }
                    } else {
                        return Err(anyhow!("No data for {:?}", path));
                    }
                },
                _ => {
                    debug!("No handler for {:?}", obj);
                    None
                },
            };
            if let Some(value) = value {
                let oid = [obj.oid.clone(), vec![1]].concat();
                debug!("insert {:?} is {:?}", oid, value);
                tree.insert(oid, value);
            }
        }
        Ok(())
    }



    pub(crate) fn get(&self, search_range: &encodings::SearchRangeList) -> Result<encodings::VarBindList> {
        self.generate_mib()?;
        let mut vbs = Vec::new();

        for search_item in search_range {
            let name = search_item.start.clone();
            let oid = as_vec(&name);
            let value = match self.mib.borrow().get(&oid) {
                Some(v) => v.clone(),
                None => encodings::Value::NoSuchObject,
            };

            vbs.push(encodings::VarBind::new(name, value));
        }
        Ok(encodings::VarBindList(vbs))
    }


    pub(crate) fn get_next(&self, search_range: &encodings::SearchRangeList) -> encodings::VarBindList {
        let mut vbs = Vec::new();
        let tree = self.mib.borrow();

        for range in search_range {
            info!("get_next: start: {:?}", range.start); // 1.3.6.1.4.1.pen
            info!("get_next: end: {:?}", range.end); // 1.3.6.1.4.1.(pen + 1)

            let bounded = match range.start.include {
                0 => Bound::Excluded,
                _ => Bound::Included,
            };
            let mut vb = encodings::VarBind::new(
                range.start.clone(), encodings::Value::EndOfMibView,
            );
            let iter = tree.range(
                (bounded(as_vec(&range.start)), Bound::Unbounded)
            );
            if let Some((oid, val)) = iter.into_iter().next() {
                let oid = encodings::ID::try_from(oid.clone()).expect("Cannot convert vec to ID.");
                if range.end.is_null() || oid < range.end {
                    vb.name = oid.clone();
                    vb.data = val.clone();
                }
            }
            vbs.push(vb);
        }

        encodings::VarBindList(vbs)
    }
}


fn run_command(command: &String) -> Result<String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow!("Command is missing"));
    }
    let mut command = Command::new(parts[0]);
    for arg in &parts[1..] {
        command.arg(arg);
    }
    debug!("Running {:?}", command);
    let output = command.output()
        .expect("failed to execute rocm-smi");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("status: {}", output.status);
    }
    let stdout = String::from_utf8(output.stdout)?;
    debug!("stdout: {}", stdout);
    Ok(stdout)
}


enum Handler {
    Literal(String),
    Json(String),
    None,
}


fn detect_handler(entry: &Entry) -> Handler {
    if let Some(l) = &entry.literal {
        Handler::Literal(l.clone())
    } else if let Some(j) = &entry.json_path {
        Handler::Json(j.clone())
    } else {
        Handler::None
    }
}

fn convert(val: MibType, converter: &str) -> Result<MibType> {
    debug!("converter: {:?}", converter);
    debug!("val: {:?}", val);
    let converters: Vec<&str> = converter
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let mut current: MibType = val;
    for converter in converters {
        debug!("converter: {:?}", converter);
        current = match converter {
            "cast_float" => {
                match &current {
                    MibType::Float(_) => current, // ignore converter
                    MibType::Integer(i) => MibType::Float(*i as f32),
                    MibType::String(s) => MibType::Float(s.parse()?)
                }
            },
            "cast_int" => {
                match &current {
                    MibType::Float(f) => MibType::Integer(f.round() as u32),
                    MibType::Integer(_) => current,  // ignore converter
                    MibType::String(s) => MibType::Integer(s.parse()?)
                }
            },
            _ if converter.starts_with("multiply_by") => {
                let c_length = converter.len();
                let factor = (&converter[12..c_length-1]).parse::<f32>()?;
                match &current {
                    MibType::Float(f) => MibType::Float(f * factor),
                    MibType::Integer(i) => MibType::Float(*i as f32 * factor),
                    MibType::String(_) => current, // ignore converter
                }
            },
            _ if converter.starts_with("trim_right") => {
                match &current {
                    MibType::Float(_) => current, // ignore converter
                    MibType::Integer(_) => current, // ignore converter
                    MibType::String(s) => {
                        let c_length = converter.len();
                        let n = (&converter[11..c_length-1]).parse::<usize>()?;
                        let s_length = s.len();
                        let s2 = &s[0..s_length-n];
                        MibType::String(s2.to_string())
                    }
                }
            },
            _ if converter.starts_with("trim") => {
                match &current {
                    MibType::Float(_) => current, // ignore converter
                    MibType::Integer(_) => current, // ignore converter
                    MibType::String(s) => {
                        let c_length = converter.len();
                        let n = (&converter[5..c_length-1]).parse::<usize>()?;
                        let s_length = s.len();
                        let s2 = &s[n..s_length-n];
                        MibType::String(s2.to_string())
                    }
                }
            },
            _ => {
                return Err(anyhow!("Unknown converter: {}", converter));
            }
        };
        debug!("current: {:?}", current);
    }
    Ok(current)
}


#[derive(Debug)]
enum MibType {
    Float(f32),
    Integer(u32),
    String(String),
}
