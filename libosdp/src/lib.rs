//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]
//! # LibOSDP - Open Supervised Device Protocol Library
//!
//! This is an open source implementation of IEC 60839-11-5 Open Supervised
//! Device Protocol (OSDP). The protocol is intended to improve interoperability
//! among access control and security products. It supports Secure Channel (SC)
//! for encrypted and authenticated communication between configured devices.
//!
//! OSDP describes the communication protocol for interfacing one or more
//! Peripheral Devices (PD) to a Control Panel (CP) over a two-wire RS-485
//! multi-drop serial communication channel. Nevertheless, this protocol can be
//! used to transfer secure data over any stream based physical channel. Read
//! more about OSDP [here][1].
//!
//! This protocol is developed and maintained by [Security Industry Association][2]
//! (SIA).
//!
//! ## Salient Features of LibOSDP
//!
//!   - Supports secure channel communication (AES-128)
//!   - Can be used to setup a PD or CP mode of operation
//!   - Exposes a well defined contract though a single header file
//!   - No run-time memory allocation. All memory is allocated at init-time
//!   - No external dependencies (for ease of cross compilation)
//!   - Fully non-blocking, asynchronous design
//!   - Provides Python3 and Rust bindings for the C library for faster
//!     testing/integration
//!
//! ## Quick start
//!
//! #### Control Panel:
//!
//! A simplified CP implementation:
//!
//! ```rust,no_run
//! use libosdp::{
//!     channel::{OsdpChannel, UnixChannel}, OsdpCommand, OsdpCommandLed,
//!     ControlPanel, OsdpError, OsdpFlag, PdInfo,
//! };
//! use std::{
//!     result::Result, thread, time::Duration,
//!     path::PathBuf, str::FromStr
//! };
//!
//! let path = PathBuf::from_str("/tmp/chan-0.sock").unwrap();
//! let stream = UnixChannel::connect(&path).unwrap();
//! let pd_info = vec![PdInfo::for_cp(
//!     "PD 101",
//!     101,
//!     115200,
//!     OsdpFlag::EnforceSecure,
//!     OsdpChannel::new::<UnixChannel>(Box::new(stream)),
//!     [
//!         0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
//!         0x0e, 0x0f,
//!     ],
//! )];
//! let mut cp = ControlPanel::new(pd_info).unwrap();
//! cp.set_event_callback(|pd, event| {
//!     println!("Received event from {pd}: {:?}", event);
//!     0
//! });
//!
//! // Send LED command to PD 0
//! cp.send_command(0, OsdpCommand::Led(OsdpCommandLed::default()));
//!
//! // From the app main loop, refresh the CP state machine
//! cp.refresh();
//! thread::sleep(Duration::from_millis(50));
//! ```
//!
//! #### Peripheral Device:
//!
//! A simplified PD implementation:
//!
//! ```rust,no_run
//! use libosdp::{
//!     channel::{OsdpChannel, UnixChannel},
//!     OsdpError, OsdpFlag, OsdpEvent, OsdpEventCardRead, PdCapEntity,
//!     PdCapability, PdId, PdInfo, PeripheralDevice,
//! };
//! use std::{result::Result, thread, time::Duration, path::PathBuf, str::FromStr};
//! let path = PathBuf::from_str("/tmp/conn-1").unwrap();
//! let stream = UnixChannel::new(&path).unwrap();
//! let pd_info = PdInfo::for_pd(
//!     "PD 101",
//!     101,
//!     115200,
//!     OsdpFlag::EnforceSecure,
//!     PdId::from_number(101),
//!     vec![PdCapability::CommunicationSecurity(PdCapEntity::new(1, 1))],
//!     OsdpChannel::new::<UnixChannel>(Box::new(stream)),
//!     [
//!         0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
//!         0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
//!     ],
//! );
//!
//! // Create a PD and setup a command callback closure.
//! let mut pd = PeripheralDevice::new(pd_info).unwrap();
//! pd.set_command_callback(|cmd| {
//!     println!("Received command {:?}", cmd);
//!     0
//! });
//!
//! // Notify the CP of an event on the PD.
//! let card_read = OsdpEventCardRead::new_weigand(16, vec![0xa1, 0xb2]).unwrap();
//! pd.notify_event(OsdpEvent::CardRead(card_read));
//!
//! // From the app main loop, refresh the PD state machine periodically
//! pd.refresh();
//! thread::sleep(Duration::from_millis(50));
//! ```
//!
//! [1]: https://libosdp.sidcha.dev/protocol/
//! [2]: https://www.securityindustry.org/industry-standards/open-supervised-device-protocol/

#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

extern crate alloc;

mod cp;
pub mod file;
#[cfg(feature = "std")]
mod pd;
mod commands;
mod events;
mod pdcap;
mod pdid;
mod pdinfo;
mod channel;

// Re-export for convenience
pub use channel::*;
pub use commands::*;
pub use events::*;
pub use pdcap::*;
pub use pdid::*;
pub use pdinfo::*;

#[allow(unused_imports)]
use alloc::{
    borrow::ToOwned, boxed::Box, ffi::CString, format, str::FromStr, string::String, sync::Arc,
    vec, vec::Vec,
};
use once_cell::sync::Lazy;
#[cfg(feature = "std")]
use thiserror::Error;

pub use cp::ControlPanel;
pub use pd::PeripheralDevice;

/// OSDP public errors
#[derive(Debug, Default)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum OsdpError {
    /// PD info error
    #[cfg_attr(feature = "std", error("Invalid PdInfo {0}"))]
    PdInfo(&'static str),

    /// Command build/send error
    #[cfg_attr(feature = "std", error("Invalid OsdpCommand"))]
    Command,

    /// Event build/send error
    #[cfg_attr(feature = "std", error("Invalid OsdpEvent"))]
    Event,

    /// PD/CP status query error
    #[cfg_attr(feature = "std", error("Failed to query {0} from device"))]
    Query(&'static str),

    /// File transfer errors
    #[cfg_attr(feature = "std", error("File transfer failed: {0}"))]
    FileTransfer(&'static str),

    /// CP/PD device setup failed.
    #[cfg_attr(feature = "std", error("Failed to setup device"))]
    Setup,

    /// String parse error
    #[cfg_attr(feature = "std", error("Type {0} parse error"))]
    Parse(String),

    /// OSDP channel error
    #[cfg_attr(feature = "std", error("Channel error: {0}"))]
    Channel(&'static str),

    /// IO Error
    #[cfg(feature = "std")]
    #[error("IO Error")]
    IO(#[from] std::io::Error),
    /// IO Error
    #[cfg(not(feature = "std"))]
    IO(Box<dyn embedded_io::Error>),

    /// Unknown error
    #[default]
    #[cfg_attr(feature = "std", error("Unknown/Unspecified error"))]
    Unknown,
}

impl From<core::convert::Infallible> for OsdpError {
    fn from(_: core::convert::Infallible) -> Self {
        unreachable!()
    }
}

impl From<ChannelError> for OsdpError {
    fn from(value: ChannelError) -> OsdpError {
        match value {
            ChannelError::WouldBlock => OsdpError::Channel("WouldBlock"),
            ChannelError::TransportError => OsdpError::Channel("TransportError"),
        }
    }
}

/// Trait to convert between BigEndian and LittleEndian types
pub trait ConvertEndian {
    /// Return `Self` as BigEndian
    fn as_be(&self) -> u32;
    /// Return `Self` as LittleEndian
    fn as_le(&self) -> u32;
}

bitflags::bitflags! {
    /// OSDP setup flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct OsdpFlag: u32 {
        /// Make security conscious assumptions where possible. Fail where these
        /// assumptions don't hold. The following restrictions are enforced in
        /// this mode:
        ///
        /// - Don't allow use of SCBK-D (implies no INSTALL_MODE)
        /// - Assume that a KEYSET was successful at an earlier time
        /// - Disallow master key based SCBK derivation
        const EnforceSecure = libosdp_sys::OSDP_FLAG_ENFORCE_SECURE;

        /// When set, the PD would allow one session of secure channel to be
        /// setup with SCBK-D.
        ///
        /// In this mode, the PD is in a vulnerable state, the application is
        /// responsible for making sure that the device enters this mode only
        /// during controlled/provisioning-time environments.
        const InstallMode = libosdp_sys::OSDP_FLAG_INSTALL_MODE;

        /// When set, CP will not error and fail when the PD sends an unknown,
        /// unsolicited response. In PD mode this flag has no use.
        const IgnoreUnsolicited = libosdp_sys::OSDP_FLAG_IGN_UNSOLICITED;
    }
}

impl FromStr for OsdpFlag {
    type Err = OsdpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "EnforceSecure" => Ok(OsdpFlag::EnforceSecure),
            "InstallMode" => Ok(OsdpFlag::InstallMode),
            "IgnoreUnsolicited" => Ok(OsdpFlag::IgnoreUnsolicited),
            _ => Err(OsdpError::Parse(format!("OsdpFlag: {s}"))),
        }
    }
}

fn cstr_to_string(s: *const ::core::ffi::c_char) -> String {
    let s = unsafe { core::ffi::CStr::from_ptr(s) };
    s.to_str().unwrap().to_owned()
}

static VERSION: Lazy<Arc<String>> = Lazy::new(|| {
    let s = unsafe { libosdp_sys::osdp_get_version() };
    Arc::new(cstr_to_string(s))
});

static SOURCE_INFO: Lazy<Arc<String>> = Lazy::new(|| {
    let s = unsafe { libosdp_sys::osdp_get_source_info() };
    Arc::new(cstr_to_string(s))
});

/// Get LibOSDP version
pub fn get_version() -> String {
    VERSION.as_ref().clone()
}

/// Get LibOSDP source info string
pub fn get_source_info() -> String {
    SOURCE_INFO.as_ref().clone()
}
