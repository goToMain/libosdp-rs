//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

//! The OSDP specification defines that communication between OSDP devices
//! happen over an RS-485 connection. For testing and development purpose this
//! can be limiting so LibOSDP defines a notion called "Channel" which is a
//! software representation (abstraction) of the physical transport medium.
//!
//! Since RS-485 is stream based protocol, we can think of it to be  something
//! that we can read from and write to (which in turn is Read and Write traits
//! in rust). This allows us to run OSDP devices over many IPC schemes such as
//! Unix socket and message queues.
//!
//! This module provides a way to define an OSDP channel and export it to
//! LibOSDP.

use core::ffi::c_void;

/// OSDP channel errors
#[derive(Clone, Debug)]
pub enum ChannelError {
    /// Channel is temporarily unavailable (could have blocked until it was
    /// ready but LibOSDP required channel to be non-blocking so return "I would
    /// have blocked" instead)
    WouldBlock,
    /// Channel failed irrecoverably.
    TransportError,
}

impl From<std::io::Error> for ChannelError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::WouldBlock => ChannelError::WouldBlock,
            _ => ChannelError::TransportError,
        }
    }
}

/// The Channel trait acts as an interface for all channel implementors. See
/// module description for the definition of a "channel" in LibOSDP.
pub trait Channel: Send + Sync {
    /// Since OSDP channels can be multi-drop (more than one PD can talk to a
    /// CP on the same channel) and LibOSDP supports mixing multi-drop
    /// connections among PDs it needs a way to identify each unique channel by
    /// a channel ID. Implementors of this trait must also provide a method
    /// which returns a unique i32 per channel.
    fn get_id(&self) -> i32;

    /// Pull as many bytes into buffer as possible; returns the number of bytes
    /// were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ChannelError>;

    /// Write a buffer into this writer, returning how many bytes were written.
    fn write(&mut self, buf: &[u8]) -> Result<usize, ChannelError>;

    /// Flush this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    fn flush(&mut self) -> Result<(), ChannelError>;
}

impl core::fmt::Debug for dyn Channel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Channel")
            .field("id", &self.get_id())
            .finish()
    }
}

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

impl From<Box<dyn Channel>> for libosdp_sys::osdp_channel {
    fn from(val: Box<dyn Channel>) -> Self {
        let id = val.get_id();
        let data = Box::into_raw(Box::new(val));
        libosdp_sys::osdp_channel {
            id,
            data: data as *mut c_void,
            recv: Some(raw_read),
            send: Some(raw_write),
            flush: Some(raw_flush),
        }
    }
}
