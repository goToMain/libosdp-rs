//
// Copyright (c) 2025 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use std::str::FromStr;
use anyhow::{bail, Error};
use nix::unistd::Pid;
pub use crate::config::config_reader::{CPConfig, PDConfig, ChannelInfo};

mod config_reader;

pub enum DeviceConfig {
    ControlPanel(CPConfig),
    PeripheralDevice(PDConfig),
}

impl DeviceConfig {
    pub fn name(&self) -> String {
    }

    pub fn get_pid(&self) -> Result<Pid, Error> {
        bail!("")
    }
    
    pub fn config_dir(self) -> PathBuf {
        let mut dir = dirs::config_dir()
            .expect("Failed to read system config directory");
        dir.push("osdp");
        _ = std::fs::create_dir_all(&dir);
        dir
    }

    pub fn runtime_dir(self) -> PathBuf {
        let mut dir = dirs::runtime_dir()
            .unwrap_or(PathBuf::from_str("/tmp").expect("Failed to read runtime directory"));
        dir.push("osdp");
        std::fs::create_dir_all(&dir)
            .expect("Failed to create runtime directory");
        dir
    }
}
