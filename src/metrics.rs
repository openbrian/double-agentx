use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Bound;
use std::process::Command;
use agentx::encodings;
use anyhow::{anyhow, Result};
use log::debug;
use serde::Deserialize;
use crate::config::{Metric};
use crate::util::{as_vec};



#[derive(Deserialize, Debug)]
struct CardData {
    #[serde(rename = "Device Name")]
    pub device_name: String,
    #[serde(rename = "Device ID")]
    pub device_id: String,
    #[serde(rename = "Device Rev")]
    pub device_rev: String,
    #[serde(rename = "Subsystem ID")]
    pub subsystem_id: String,
    #[serde(rename = "GUID")]
    pub guid: String,
    #[serde(rename = "Unique ID")]
    pub unique_id: String,
    #[serde(rename = "VBIOS version")]
    pub vbios_version: String,
    #[serde(rename = "Temperature (Sensor edge) (C)")]
    pub temperature_sensor_edge: String,  // yes it's a string
    #[serde(rename = "Temperature (Sensor junction) (C)")]
    pub temperature_sensor_junction: String,
    #[serde(rename = "Temperature (Sensor memory) (C)")]
    pub temperature_sensor_memory: String,
}


#[derive(Deserialize, Debug)]
struct RocmSmi {
    #[serde(rename = "card0")]
    card_0: CardData,
    // This is not used yet.
    // system: HashMap<String, String>,
}


pub(crate) struct Metrics {
    // mib: BTreeMap<encodings::ID, encodings::Value>,
    mib: RefCell<BTreeMap<Vec<u32>, encodings::Value>>,
    // TODO: Consider not having BTree of oids, but the oid tree itself.  It
    // will be harder to get ranges.
    // TODO: Consider prepopulating this with the number of instances needed.
    // There's no need to recreate oids.
    oid_base: Vec<u32>,
    config: Metric,
}


impl Metrics {
    pub(crate) fn new(oid_base: &Vec<u32>, config: &Metric) -> Self {
        debug!("Creating new Metrics");
        debug!("oid_base: {:?}", oid_base);
        debug!("config: {:?}", config);
        Self {
            mib: RefCell::new(BTreeMap::new()),
            oid_base: [oid_base.to_owned(), config.relative_oid.clone()].concat(),
            config: config.clone(),
        }
    }


    fn generate_mib(&self) -> Result<()> {
        // Currently we get all info.  Not sure if getting any specific data
        // is costly, but if so, we can break this up into showfan, showpower,
        // showtemp, etc.
        let parts: Vec<&str> = self.config.command.split_whitespace().collect();
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        // debug!("stdout: {}", stdout);

        let smi_data: RocmSmi = serde_json::from_str(&stdout).expect("failed to parse json");
        // debug!("{:?}", smi_data);

        let mut tree = self.mib.borrow_mut();
        tree.clear();

        // TODO: Given the oid_base, generate all the other oids once.
        let meta_prefix = [self.oid_base.to_owned(), vec![1]].concat();

        tree.insert(
            [meta_prefix.clone(), vec![1]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    "1.0".to_string(),
                )
            ),
        );

        let resource_prefix = [self.oid_base.to_owned(), vec![2, 1]].concat();

        let instance = 1u32;

        // Identify the instance we have here.
        tree.insert(
            [resource_prefix.clone(), vec![MIB::Minor as u32, instance]].concat(),
            encodings::Value::Integer(instance as i32),
        );

        tree.insert(
            [resource_prefix.clone(), vec![MIB::DeviceName as u32, instance]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    smi_data.card_0.device_name.to_string(),
                )
            ),
        );
        tree.insert(
            [resource_prefix.clone(), vec![MIB::DeviceId as u32, instance]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    smi_data.card_0.device_id.to_string(),
                )
            ),
        );
        tree.insert(
            [resource_prefix.clone(), vec![MIB::DeviceRev as u32, instance]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    smi_data.card_0.device_rev.to_string(),
                )
            ),
        );
        tree.insert(
            [resource_prefix.clone(), vec![MIB::SubsystemId as u32, instance]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    smi_data.card_0.subsystem_id.to_string(),
                )
            ),
        );
        tree.insert(
            [resource_prefix.clone(), vec![MIB::GUID as u32, instance]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    smi_data.card_0.guid.to_string(),
                )
            ),
        );
        tree.insert(
            [resource_prefix.clone(), vec![MIB::UniqueId as u32, instance]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    smi_data.card_0.unique_id.to_string(),
                )
            ),
        );
        tree.insert(
            [resource_prefix.clone(), vec![MIB::VbiosVersion as u32, instance]].concat(),
            encodings::Value::OctetString(
                encodings::OctetString(
                    smi_data.card_0.vbios_version.to_string(),
                )
            ),
        );

        let temp: f64 = smi_data.card_0.temperature_sensor_edge.parse()?;
        tree.insert(
            [resource_prefix.clone(), vec![MIB::TemperatureSensorEdge as u32, instance]].concat(),
            encodings::Value::Integer((temp * 1000.0) as i32),
        );

        let temp: f64 = smi_data.card_0.temperature_sensor_junction.parse()?;
        tree.insert(
            [resource_prefix.clone(), vec![MIB::TemperatureSensorJunction as u32, instance]].concat(),
            encodings::Value::Integer((temp * 1000.0) as i32),
        );

        let temp: f64 = smi_data.card_0.temperature_sensor_memory.parse()?;
        tree.insert(
            [resource_prefix.clone(), vec![MIB::TemperatureSensorMemory as u32, instance]].concat(),
            encodings::Value::Integer((temp * 1000.0) as i32),
        );
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
            debug!("get_next: start: {:?}", range.start); // 1.3.6.1.4.1.pen
            debug!("get_next: end: {:?}", range.end); // 1.3.6.1.4.1.(pen + 1)

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


#[allow(clippy::upper_case_acronyms)]
enum MIB {
    Minor = 1,

    DeviceName,
    DeviceId,
    DeviceRev,
    SubsystemId,
    GUID,
    UniqueId,
    VbiosVersion,
    TemperatureSensorEdge,
    TemperatureSensorJunction,
    TemperatureSensorMemory,
}
