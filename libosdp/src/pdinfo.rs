//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use alloc::ffi::CString;
use core::ffi::c_void;

use super::{OsdpFlag, PdCapability, PdId, Channel, ChannelError};

unsafe extern "C" fn raw_read(data: *mut c_void, buf: *mut u8, len: i32) -> i32 {
    let channel: *mut Box<dyn Channel> = data as *mut _;
    let channel = channel.as_mut().unwrap();
    let mut read_buf = vec![0u8; len as usize];
    match channel.read(&mut read_buf) {
        Ok(n) => {
            let src_ptr = read_buf.as_mut_ptr();
            core::ptr::copy_nonoverlapping(src_ptr, buf, len as usize);
            n as i32
        }
        Err(ChannelError::WouldBlock) => 0,
        Err(_) => -1,
    }
}

unsafe extern "C" fn raw_write(data: *mut c_void, buf: *mut u8, len: i32) -> i32 {
    let channel: *mut Box<dyn Channel> = data as *mut _;
    let channel = channel.as_mut().unwrap();
    let mut write_buf = vec![0u8; len as usize];
    core::ptr::copy_nonoverlapping(buf, write_buf.as_mut_ptr(), len as usize);
    match channel.as_mut().write(&write_buf) {
        Ok(n) => n as i32,
        Err(ChannelError::WouldBlock) => 0,
        Err(_) => -1,
    }
}

unsafe extern "C" fn raw_flush(data: *mut c_void) {
    let channel: *mut Box<dyn Channel> = data as *mut _;
    let channel = channel.as_mut().unwrap();
    let _ = channel.as_mut().flush();
}

fn into_osdp_channel(channel: Box<dyn Channel>) -> libosdp_sys::osdp_channel {
    let id = channel.get_id();
    let data = Box::into_raw(Box::new(channel));
    libosdp_sys::osdp_channel {
        id,
        data: data as *mut c_void,
        recv: Some(raw_read),
        send: Some(raw_write),
        flush: Some(raw_flush),
    }
}

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
    scbk: [u8; 16],
}

impl PdInfo {
    /// Create an instance of PdInfo struct for Peripheral Device (PD)
    ///
    /// # Arguments
    ///
    /// * `name` - User provided name for this PD (log messages include this name)
    /// * `address` - 7 bit PD address. the rest of the bits are ignored. The
    ///   special address 0x7F is used for broadcast. So there can be 2^7-1
    ///   devices on a multi-drop channel
    /// * `baud_rate` - Can be one of 9600/19200/38400/57600/115200/230400
    /// * `flags` - Used to modify the way the context is setup.
    /// * `id` - Static information that the PD reports to the CP when it
    ///    received a `CMD_ID`. These information must be populated by a PD
    ///    application.
    /// * `cap` - A vector of [`PdCapability`] entries (PD mode)
    /// * `channel` - Osdp communication channel.
    /// * `scbk` - Secure channel base key data
    pub fn for_pd(
        name: &str,
        address: i32,
        baud_rate: i32,
        flags: OsdpFlag,
        id: PdId,
        cap: Vec<PdCapability>,
        channel: Box<dyn Channel>,
        scbk: [u8; 16],
    ) -> Self {
        let name = CString::new(name).unwrap();
        let cap = cap.into_iter().map(|c| c.into()).collect();
        Self {
            name,
            address,
            baud_rate,
            flags,
            id,
            cap,
            channel: Some(channel),
            scbk,
        }
    }

    /// Create an instance of PdInfo struct for Control Panel (CP)
    ///
    /// # Arguments
    ///
    /// * `name` - User provided name for this PD (log messages include this name)
    /// * `address` - 7 bit PD address. the rest of the bits are ignored. The
    ///   special address 0x7F is used for broadcast. So there can be 2^7-1
    ///   devices on a multi-drop channel
    /// * `baud_rate` - Can be one of 9600/19200/38400/57600/115200/230400
    /// * `flags` - Used to modify the way the context is setup.
    /// * `channel` - Osdp communication channel.
    /// * `scbk` - Secure channel base key data
    pub fn for_cp(
        name: &str,
        address: i32,
        baud_rate: i32,
        flags: OsdpFlag,
        channel: Box<dyn Channel>,
        scbk: [u8; 16],
    ) -> Self {
        let name = CString::new(name).unwrap();
        Self {
            name,
            address,
            baud_rate,
            flags,
            id: PdId::default(),
            cap: vec![],
            channel: Some(channel),
            scbk,
        }
    }

    pub fn as_struct(&mut self) -> libosdp_sys::osdp_pd_info_t {
        let channel = into_osdp_channel(self.channel.take().unwrap());
        libosdp_sys::osdp_pd_info_t {
            name: self.name.as_ptr(),
            baud_rate: self.baud_rate,
            address: self.address,
            flags: self.flags.bits() as i32,
            id: self.id.clone().into(),
            cap: self.cap.as_ptr(),
            channel,
            scbk: self.scbk.as_ptr(),
        }
    }
}
