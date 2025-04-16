use std::time::Duration;
use anyhow::Error;
use serialport::SerialPort;
use libosdp::{Channel, ChannelError};
use crate::channel::str_to_channel_id;

pub struct SerialChannel {
    id: i32,
    device: Box<dyn SerialPort>,
    port_name: String,
    baud_rate: u32,
}

impl SerialChannel {
    pub fn open(port_name: &str, baud_rate: u32) -> Result<Self, Error> {
        let device = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(10)) // Set a read timeout
            .open()?;
        Ok(Self {
            id: str_to_channel_id(port_name),
            device,
            port_name: port_name.to_owned(),
            baud_rate,
        })
    }
}

impl Channel for SerialChannel {
    fn get_id(&self) -> i32 {
        self.id
    }

    fn read(&mut self, buf: &mut [u8]) -> std::prelude::v1::Result<usize, ChannelError> {
        self.device.read(buf).map_err(ChannelError::from)
    }

    fn write(&mut self, buf: &[u8]) -> std::prelude::v1::Result<usize, ChannelError> {
        self.device.write(buf).map_err(ChannelError::from)
    }

    fn flush(&mut self) -> std::prelude::v1::Result<(), ChannelError> {
        self.device.flush().map_err(ChannelError::from)
    }
}
