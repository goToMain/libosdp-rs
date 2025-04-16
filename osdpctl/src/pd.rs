//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{thread, time::Duration};

use anyhow::Context;
use libosdp::{Channel, OsdpCommand, OsdpFlag, PdCapEntity, PdCapability, PdInfoBuilder, PeripheralDevice};
use std::io::Write;
use crate::channel::serial::SerialChannel;
use crate::channel::unix::UnixChannel;
use crate::config::{ChannelInfo, PDConfig};

type Result<T> = anyhow::Result<T, anyhow::Error>;

fn setup(dev: &PDConfig, daemonize: bool) -> Result<()> {
    if dev.runtime_dir.exists() {
        std::fs::remove_dir_all(&dev.runtime_dir)?;
    }
    std::fs::create_dir_all(&dev.runtime_dir)?;
    if daemonize {
        crate::daemonize::daemonize(&dev.runtime_dir, &dev.name)?;
    } else {
        let pid_file = dev.runtime_dir.join(format!("dev-{}.pid", dev.name));
        let mut pid_file = std::fs::File::create(pid_file)?;
        write!(pid_file, "{}", std::process::id())?;
    }
    Ok(())
}

pub fn main(config: PDConfig, daemonize: bool) -> Result<()> {
    setup(&config, daemonize)?;
    let channel: Box<dyn Channel> = match config.channel {
        ChannelInfo::Serial(port,speed) => Box::new(SerialChannel::open(&port, speed)?),
        ChannelInfo::Unix(path) => Box::new(UnixChannel::connect(&path)?),
    };
    let pd_info = PdInfoBuilder::new()
        .name(&config.name)?
        .address(config.address)?
        .baud_rate()?
        .flag(OsdpFlag::EnforceSecure)
        .capability(PdCapability::CommunicationSecurity(PdCapEntity::new(1, 1)))
        .secure_channel_key(config.scbk);
    let mut pd = libosdp::PeripheralDevice::new(pd_info, channel)?;
    
    let (channel, pd_info) = config.pd_info().context("Failed to create PD info")?;
    let mut pd = PeripheralDevice::new(pd_info, channel)?;
    pd.set_command_callback(|command| {
        match command {
            OsdpCommand::Led(c) => {
                log::info!("Command: {:?}", c);
            }
            OsdpCommand::Buzzer(c) => {
                log::info!("Command: {:?}", c);
            }
            OsdpCommand::Text(c) => {
                log::info!("Command: {:?}", c);
            }
            OsdpCommand::Output(c) => {
                log::info!("Command: {:?}", c);
            }
            OsdpCommand::ComSet(c) => {
                log::info!("Command: {:?}", c);
            }
            OsdpCommand::KeySet(c) => {
                log::info!("Command: {:?}", c);
                let mut key = [0; 16];
                key.copy_from_slice(&c.data[0..16]);
                config.key_store.store(key).unwrap();
            }
            OsdpCommand::Mfg(c) => {
                log::info!("Command: {:?}", c);
            }
            OsdpCommand::FileTx(c) => {
                log::info!("Command: {:?}", c);
            }
            OsdpCommand::Status(c) => {
                log::info!("Command: {:?}", c);
            }
        }
        0
    });
    loop {
        pd.refresh();
        thread::sleep(Duration::from_millis(50));
    }
}
