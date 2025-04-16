//
// Copyright (c) 2025 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::channel;
use anyhow::{bail, Error};
use log::LevelFilter;
use serde::Deserialize;
use toml::Value;
use libosdp::{OsdpFlag, PdCapEntity, PdCapability, PdId};
use crate::keystore::KeyStore;

#[derive(Debug, Deserialize)]
struct RawChannel {
    path: String,
    speed: Option<u32>,
    r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct RawPDConfigReader {
    pub name: String,
    pub address: i32,
    pub channel: RawChannel,
    pub scbk: String,
    pub flags: Option<String>,
    pub log_level: Option<String>,
    pub capability: Option<Value>, // Store as TOML Value if structure varies
    pub pd_id: Option<Value>, // Store as TOML Value if structure varies
}

#[derive(Debug, Deserialize)]
pub struct RawCPConfigReader {
    pub name: String,
    pub log_level: Option<String>,
    pub pd: Vec<RawPDConfigReader>,
}

impl RawCPConfigReader {
    pub fn load(file: &PathBuf) -> Result<RawCPConfigReader, Error> {
        let config_content = fs::read_to_string(&file)
            .expect("error reading config file");

        let toml_value: Value = toml::from_str(&config_content).unwrap();

        if toml_value.get("pd").is_some() {
            let cp_config: RawCPConfigReader = toml::from_str(&config_content).unwrap();
            Ok(cp_config)
        } else {
            bail!("CP config must contain at least one `PD` entry");
        }
    }
}

impl RawPDConfigReader {
    pub fn load(file: &PathBuf) -> Result<RawPDConfigReader, Error> {
        let config_content = fs::read_to_string(&file)
            .expect("error reading config file");

        let toml_value: Value = toml::from_str(&config_content).unwrap();

        if toml_value.get("pd_id").is_none() {
            let pd_config: RawPDConfigReader = toml::from_str(&config_content).unwrap();
            Ok(pd_config)
        } else {
            bail!("PD config file must contain `pd_id` entry")
        }
    }
}

#[test]
fn test_raw_config_load() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(project_root).join("config/pd-0.toml");
    println!("Parsed as PD Config: {:?}", RawPDConfigReader::load(&path));

    let path = PathBuf::from(project_root).join("config/cp-multiple-pd.toml");
    println!("Parsed as CP Config: {:?}", RawCPConfigReader::load(&path));
}

#[derive(Debug)]
pub enum ChannelInfo {
    Serial(String, u32),
    Unix(PathBuf),
}

#[derive(Debug)]
pub struct PDConfig {
    pub name: String,
    pub address: i32,
    pub channel: ChannelInfo,
    pub scbk: [u8; 16],
    pub flags: OsdpFlag,
    pub log_level: LevelFilter,
    pub capability: Vec<PdCapability>,
    pub pd_id: PdId,
}

#[derive(Debug)]
pub struct PDData {
    pub name: String,
    pub address: i32,
    pub channel: ChannelInfo,
    pub scbk: [u8; 16],
    pub flags: OsdpFlag,
}

impl From<RawChannel> for ChannelInfo {
    fn from(channel: RawChannel) -> Self {
        match channel.r#type.as_str() {
            "unix" => ChannelInfo::Unix(PathBuf::from(channel.path)),
            "serial" => ChannelInfo::Serial(channel.path, channel.speed.unwrap_or(115200)),
            _ => panic!("invalid channel type"),
        }
    }
}

#[derive(Debug)]
pub struct CPConfig {
    pub name: String,
    pub log_level: LevelFilter,
    pub pd: Vec<PDData>,
}

impl Into<PDData> for RawPDConfigReader {
    fn into(self) -> PDData {
        let flags = self.flags.map_or(OsdpFlag::empty(), |s| OsdpFlag::from_str(&s)
            .expect("Invalid flags entry; key `flags` must be a list of type `OsdpFlag`"));
        PDData {
            name: self.name.clone(),
            address: self.address,
            channel: self.channel.into(),
            scbk: KeyStore::str_to_key(&self.scbk).unwrap(),
            flags,
        }
    }
}

impl Into<CPConfig> for RawCPConfigReader {
    fn into(self) -> CPConfig {
        let log_level = self.log_level.map_or(LevelFilter::Info, |l| LevelFilter::from_str(&l)
            .expect("Invalid LogLevel entry"));
        CPConfig {
            name: self.name.clone(),
            log_level,
            pd: self.pd.into_iter().map(|p| p.into()).collect(),
        }
    }
}

impl From<RawPDConfigReader> for PDConfig {
    fn from(raw: RawPDConfigReader) -> PDConfig {
        let capability = raw.capability.map_or(Vec::new(), |cap| {
            cap.as_table().map_or(Vec::new(), | table | {
                table.iter().map(|(key, value)| {
                    let comp = value.get("Compliance").and_then(Value::as_integer)
                        .expect("Invalid capability entry; key `Compliance` missing") as u8;
                    let num = value.get("NumItems").and_then(Value::as_integer)
                        .expect("Invalid capability entry; key `NumItems` missing") as u8;
                    PdCapability::from_str(key, PdCapEntity::new(comp, num))
                        .expect("Invalid capability entry; Unknown capability name")
                }).collect()
            })
        });
        let flags = raw.flags.map_or_else(OsdpFlag::empty, |flags| OsdpFlag::from_str(&flags)
            .expect("Invalid flags entry; key `flags` must be a list of type `OsdpFlag`"));
        let log_level = raw.log_level.map_or(LevelFilter::Info, |l| LevelFilter::from_str(&l)
            .expect("Invalid LogLevel entry"));
        let scbk = KeyStore::str_to_key(&raw.scbk)
            .expect("Invalid scbk");
        PDConfig {
            name: raw.name.clone(),
            address: raw.address,
            channel: ChannelInfo::from(raw.channel),
            scbk,
            flags,
            log_level,
            capability,
            pd_id: Default::default(),
        }
    }
}

impl CPConfig {
    pub fn load(path: &PathBuf) -> Result<CPConfig, Error> {
        Ok(RawCPConfigReader::load(path)?.into())
    }
}

impl PDConfig {
    pub fn load(path: &PathBuf) -> Result<PDConfig, Error> {
        Ok(RawPDConfigReader::load(path)?.into())
    }
}

#[test]
fn test_config() {
    let project_root = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(project_root).join("config/pd-0.toml");
    println!("Parsed as PD Config: {:?}", PDConfig::load(&path));

    let path = PathBuf::from(project_root).join("config/cp-multiple-pd.toml");
    println!("Parsed as CP Config: {:?}", CPConfig::load(&path));
}