//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{thread, time::Duration};
use libosdp::OsdpEvent;
use std::io::Write;
use crate::config::{CPConfig, ChannelInfo};
use crate::channel::serial::SerialChannel;
use crate::channel::unix::UnixChannel;

type Result<T> = anyhow::Result<T, anyhow::Error>;

fn setup(config: &CPConfig, daemonize: bool) -> Result<()> {
    if config.runtime_dir.exists() {
        std::fs::remove_dir_all(&config.runtime_dir)?;
    }
    std::fs::create_dir_all(&config.runtime_dir)?;
    if daemonize {
        crate::daemonize::daemonize(&config.runtime_dir, &config.name)?;
    } else {
        let pid_file = config.runtime_dir.join(format!("dev-{}.pid", config.name));
        let mut pid_file = std::fs::File::create(pid_file)?;
        write!(pid_file, "{}", std::process::id())?;
    }
    Ok(())
}

pub fn main(config: CPConfig, daemonize: bool) -> Result<()> {
    setup(&config, daemonize)?;

    let mut cp = libosdp::ControlPanelBuilder::new();
    for pd in config.pd {
        let pd_info = libosdp::PdInfoBuilder::new()
            .name(&pd.name)?
            .address(pd.address)?
            .flag(pd.flags)
            .secure_channel_key(pd.scbk);
        let channel: Box<dyn libosdp::Channel> = match pd.channel {
            ChannelInfo::Serial(path, speed) => Box::new(SerialChannel::open(&path, speed)?),
            ChannelInfo::Unix(path) => Box::new(UnixChannel::connect(&path)?),
        };
        cp = cp.add_channel(channel, vec![pd_info]);
    }

    let mut cp = cp.build()?;
    cp.set_event_callback(|pd, event| {
        match event {
            OsdpEvent::CardRead(e) => {
                log::info!("Event: PD-{pd} {:?}", e);
            }
            OsdpEvent::KeyPress(e) => {
                log::info!("Event: PD-{pd} {:?}", e);
            }
            OsdpEvent::MfgReply(e) => {
                log::info!("Event: PD-{pd} {:?}", e);
            }
            OsdpEvent::Status(e) => {
                log::info!("Event: PD-{pd} {:?}", e);
            }
        }
        0
    });
    loop {
        cp.refresh();
        thread::sleep(Duration::from_millis(50));
    }
}
