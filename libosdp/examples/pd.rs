//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use libosdp::{
    Channel, ChannelError, OsdpError, OsdpFlag, PdCapEntity, PdCapability, PdId, PdInfo,
};
use std::{thread, time::Duration};

struct OsdpChannel;

impl OsdpChannel {
    pub fn new(_path: &str) -> Self {
        // setup device
        Self {
        }
    }
}

/// Read documentation for each member in [libosdp::Channel].
impl Channel for OsdpChannel {
    fn get_id(&self) -> i32 {
        0
    }

    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ChannelError> {
        // TODO: Read from device
        Ok(0)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, ChannelError> {
        // TODO: Write from device
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), ChannelError> {
        // TODO: flush device
        Ok(())
    }
}

fn main() -> Result<(), OsdpError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .format_timestamp(None)
        .init();
    let args: Vec<String> = std::env::args().collect();
    let channel = OsdpChannel::new(&args[1]);
    let pd_info = PdInfo::for_pd(
        "PD 101",
        101,
        115200,
        OsdpFlag::EnforceSecure,
        PdId::from_number(101),
        vec![PdCapability::CommunicationSecurity(PdCapEntity::new(1, 1))],
        Box::new(channel),
        [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ],
    );
    let mut pd = libosdp::PeripheralDevice::new(pd_info)?;
    pd.set_command_callback(|_| {
        println!("Received command!");
        0
    });
    loop {
        pd.refresh();
        thread::sleep(Duration::from_millis(50));
    }
}
