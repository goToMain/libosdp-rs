//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use libosdp::{
    OsdpError, OsdpFlag, PdInfo, Channel, ChannelError,
};
use std::{env, thread, time::Duration};

struct OsdpChannel;

impl OsdpChannel {
    fn new(_path: &str) -> Self {
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
    let args: Vec<String> = env::args().collect();
    let channel = OsdpChannel::new(&args[1]);
    let pd_info = vec![PdInfo::for_cp(
        "PD 101",
        101,
        115200,
        OsdpFlag::EnforceSecure,
        Box::new(channel),
        [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ],
    )];
    let mut cp = libosdp::ControlPanel::new(pd_info)?;
    loop {
        cp.refresh();
        thread::sleep(Duration::from_millis(50));
    }
}
