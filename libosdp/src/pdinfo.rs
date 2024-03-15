//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use alloc::ffi::CString;

use crate::{Channel, OsdpError, OsdpFlag, PdCapability, PdId};

/// OSDP PD Information. This struct is used to describe a PD to LibOSDP
#[derive(Debug)]
pub struct PdInfo {
    name: CString,
    address: i32,
    baud_rate: i32,
    flags: OsdpFlag,
    id: PdId,
    cap: Vec<libosdp_sys::osdp_pd_cap>,
    channel: Option<Box<dyn Channel>>,
    scbk: Option<[u8; 16]>,
}

/// OSDP PD Info Builder
#[derive(Debug, Default)]
pub struct PdInfoBuilder {
    name: Option<CString>,
    address: i32,
    baud_rate: i32,
    flags: OsdpFlag,
    id: PdId,
    cap: Vec<libosdp_sys::osdp_pd_cap>,
    channel: Option<Box<dyn Channel>>,
    scbk: Option<[u8; 16]>,
}

impl PdInfoBuilder {
    /// Create am instance of PdInfo builder
    pub fn new() -> PdInfoBuilder {
        PdInfoBuilder::default()
    }

    /// Set PD name; a user provided name for this PD (log messages include this
    /// name defaults to pd-<address>)
    pub fn name(mut self, name: &str) -> Result<PdInfoBuilder, OsdpError> {
        let name = CString::new(name).map_err(|_| OsdpError::PdInfoBuilder("invalid name"))?;
        self.name = Some(name);
        Ok(self)
    }

    /// Set 7 bit PD address; the special address 0x7F is used for broadcast. So
    /// there can be 2^7-1 valid addresses on a bus.
    pub fn address(mut self, address: i32) -> Result<PdInfoBuilder, OsdpError> {
        if address > 126 {
            return Err(OsdpError::PdInfoBuilder("invalid address"));
        }
        self.address = address;
        Ok(self)
    }

    /// Set baud rate; can be one of 9600/19200/38400/57600/115200/230400
    pub fn baud_rate(mut self, baud_rate: i32) -> Result<PdInfoBuilder, OsdpError> {
        if baud_rate != 9600
            && baud_rate != 19200
            && baud_rate != 38400
            && baud_rate != 57600
            && baud_rate != 115200
            && baud_rate != 230400
        {
            return Err(OsdpError::PdInfoBuilder("invalid baud rate"));
        }
        self.baud_rate = baud_rate;
        Ok(self)
    }

    /// Set flags for the PD; used to modify the way the context is setup
    pub fn flag(mut self, flag: OsdpFlag) -> PdInfoBuilder {
        self.flags.set(flag, true);
        self
    }

    /// Set PD ID; Static information that the PD reports to the CP when it
    /// received a `CMD_ID`. For CP mode, this field is ignored, but PD mode
    /// must set
    pub fn id(mut self, id: PdId) -> PdInfoBuilder {
        self.id = id;
        self
    }

    /// Set a PD capability
    pub fn capability(mut self, cap: PdCapability) -> PdInfoBuilder {
        self.cap.push(cap.into());
        self
    }

    /// Set Osdp communication channel
    pub fn channel(mut self, channel: Box<dyn Channel>) -> PdInfoBuilder {
        self.channel = Some(channel);
        self
    }

    /// Set secure channel key. If the key is not set, the PD will be be set to
    /// install mode.
    pub fn secure_channel_key(mut self, key: [u8; 16]) -> PdInfoBuilder {
        self.scbk = Some(key);
        self
    }

    /// Finalize the PdInfo from the current builder
    pub fn build(self) -> PdInfo {
        let name = self
            .name
            .unwrap_or_else(|| CString::new(format!("PD-{}", self.address)).unwrap());
        PdInfo {
            name,
            address: self.address,
            baud_rate: self.baud_rate,
            flags: self.flags,
            id: self.id,
            cap: self.cap,
            channel: self.channel,
            scbk: self.scbk,
        }
    }
}

impl PdInfo {
    /// Get a C-repr struct for PdInfo that LibOSDP can operate on.
    pub fn as_struct(&mut self) -> libosdp_sys::osdp_pd_info_t {
        let scbk;
        if let Some(key) = self.scbk.as_mut() {
            scbk = key.as_mut_ptr();
        } else {
            scbk = std::ptr::null_mut::<u8>();
        }
        libosdp_sys::osdp_pd_info_t {
            name: self.name.as_ptr(),
            baud_rate: self.baud_rate,
            address: self.address,
            flags: self.flags.bits() as i32,
            id: self.id.clone().into(),
            cap: self.cap.as_ptr(),
            channel: self.channel.take().unwrap().into(),
            scbk,
        }
    }
}
