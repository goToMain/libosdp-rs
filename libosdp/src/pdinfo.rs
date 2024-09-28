//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::{OsdpError, OsdpFlag, PdCapability, PdId};
use alloc::{boxed::Box, ffi::CString, format, string::String, vec::Vec};
use core::ops::Deref;

/// OSDP PD Information. This struct is used to describe a PD to LibOSDP
#[derive(Debug, Default)]
pub struct PdInfo {
    name: CString,
    address: i32,
    baud_rate: i32,
    flags: OsdpFlag,
    id: PdId,
    cap: Vec<libosdp_sys::osdp_pd_cap>,
    channel: Option<libosdp_sys::osdp_channel>,
    scbk: Option<[u8; 16]>,
}
impl PdInfo {
    /// Gets the PDs `name`
    /// A user provided `name` for this PD (log messages include this name defaults to `pd-<address>`)
    ///
    /// # Example
    /// ```
    /// # use libosdp::PdInfoBuilder;
    /// let pd = PdInfoBuilder::new().name("door_42").unwrap().build();
    /// assert_eq!(pd.name(), "door_42".to_string());
    /// ```
    #[must_use]
    pub fn name(&self) -> String {
        self.name
            .clone()
            .into_string()
            .expect("since this is configured with a &str, this must be valid String")
    }
    /// Gets the PDs 7 bit `address`
    /// The special address 0x7F is used for broadcast.
    /// So there can be 2^7-1 valid addresses on a bus.
    ///
    /// # Example
    /// ```
    /// # use libosdp::PdInfoBuilder;
    /// let pd = PdInfoBuilder::new().address(42).unwrap().build();
    /// assert_eq!(pd.address(), 42);
    /// ```
    #[must_use]
    pub fn address(&self) -> i32 {
        self.address
    }

    /// Gets the PDs baud rate.
    /// Can be one of `9600`/`19200`/`38400`/`57600`/`115200`/`230400`
    ///
    /// # Example
    /// ```
    /// # use libosdp::PdInfoBuilder;
    /// let pd = PdInfoBuilder::new().baud_rate(9600).unwrap().build();
    /// assert_eq!(pd.baud_rate(), 9600);
    /// ```
    pub fn baud_rate(&self) -> i32 {
        self.baud_rate
    }

    /// Gets the PDs [`OsdpFlag`]
    /// Used to modify the way the context is set up
    ///
    /// # Example
    /// ```
    /// # use libosdp::{OsdpFlag, PdInfoBuilder};
    /// let pd = PdInfoBuilder::new().flag(OsdpFlag::EnforceSecure).build();
    /// assert_eq!(pd.flag(), OsdpFlag::EnforceSecure);
    /// ```
    #[must_use]
    pub fn flag(&self) -> OsdpFlag {
        self.flags
    }

    /// Gets the PDs' [`PdId`]
    /// Static information that the PD reports to the CP when it received a `CMD_ID`.
    /// For CP mode, this field is ignored, but PD mode must set it
    ///
    /// # Example
    /// ```
    /// # use libosdp::{PdId, PdInfoBuilder};
    /// let pd = PdInfoBuilder::new().id(&PdId::from_number(42)).build();
    /// assert_eq!(pd.id(), PdId::from_number(42));
    /// ```
    #[must_use]
    pub fn id(&self) -> PdId {
        self.id
    }

    /// Get a PDs [`PdCapability`]s
    ///
    /// # Example
    /// ```
    /// # use libosdp::{PdCapability, PdInfoBuilder, PdCapEntity};
    /// let pd = PdInfoBuilder::new()
    ///             .capability(PdCapability::CommunicationSecurity(PdCapEntity::new(1, 1)))
    ///             .capability(PdCapability::AudibleOutput(PdCapEntity::new(1, 1)))
    ///             .build();
    /// assert_eq!(
    ///   pd.capabilities(),
    ///   vec![PdCapability::CommunicationSecurity(PdCapEntity::new(1, 1)), PdCapability::AudibleOutput(PdCapEntity::new(1, 1))]
    /// );
    /// ```
    #[must_use]
    pub fn capabilities(&self) -> Vec<PdCapability> {
        self.cap.iter().cloned().map(PdCapability::from).collect()
    }

    /// Get a PDs secure channel key.
    /// If the key is not set, the PD will be set to install mode.
    ///
    /// # Example
    /// ```
    /// # use libosdp::PdInfoBuilder;
    /// # #[rustfmt::skip]
    /// # let pd_0_key = [
    /// #   0x94, 0x4b, 0x8e, 0xdd, 0xcb, 0xaa, 0x2b, 0x5f,
    /// #   0xe2, 0xb0, 0x14, 0x8d, 0x1b, 0x2f, 0x95, 0xc9
    /// # ];
    /// let pd = PdInfoBuilder::new().secure_channel_key(pd_0_key).build();
    /// assert_eq!(pd.secure_channel_key(), Some(pd_0_key));
    /// ```

    #[must_use]
    pub fn secure_channel_key(&self) -> Option<[u8; 16]> {
        self.scbk
    }
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
    channel: Option<libosdp_sys::osdp_channel>,
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

    /// Set baud rate; can be one of `9600`/`19200`/`38400`/`57600`/`115200`/`230400`
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
    /// must set it
    pub fn id(mut self, id: &PdId) -> PdInfoBuilder {
        self.id = *id;
        self
    }

    /// Set a PD capability
    pub fn capability(mut self, cap: PdCapability) -> PdInfoBuilder {
        self.cap.push(cap.into());
        self
    }

    /// Set multiple capabilities at once
    pub fn capabilities<'a, I>(mut self, caps: I) -> PdInfoBuilder
    where
        I: IntoIterator<Item = &'a PdCapability>,
    {
        for cap in caps {
            self.cap.push(cap.clone().into());
        }
        self
    }

    /// Set Osdp communication channel
    pub fn channel(mut self, channel: libosdp_sys::osdp_channel) -> PdInfoBuilder {
        self.channel = Some(channel);
        self
    }

    /// Set secure channel key. If the key is not set, the PD will be set to
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

#[repr(transparent)]
pub(crate) struct OsdpPdInfoHandle(pub libosdp_sys::osdp_pd_info_t);

impl Deref for OsdpPdInfoHandle {
    type Target = libosdp_sys::osdp_pd_info_t;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PdInfo> for OsdpPdInfoHandle {
    fn from(info: PdInfo) -> OsdpPdInfoHandle {
        let scbk = if let Some(key) = info.scbk {
            Box::into_raw(Box::new(key)) as *mut _
        } else {
            core::ptr::null_mut::<u8>()
        };
        let cap = if !info.cap.is_empty() {
            let mut cap = info.cap.clone();
            cap.reserve(1);
            cap.push(libosdp_sys::osdp_pd_cap {
                function_code: -1i8 as u8,
                compliance_level: 0,
                num_items: 0,
            });
            Box::into_raw(cap.into_boxed_slice()) as *mut _
        } else {
            core::ptr::null_mut::<libosdp_sys::osdp_pd_cap>()
        };
        OsdpPdInfoHandle(libosdp_sys::osdp_pd_info_t {
            name: info.name.clone().into_raw(),
            baud_rate: info.baud_rate,
            address: info.address,
            flags: info.flags.bits() as i32,
            id: info.id.into(),
            cap: cap as *mut _,
            channel: info.channel.expect("no channel provided"),
            scbk,
        })
    }
}

impl Drop for OsdpPdInfoHandle {
    fn drop(&mut self) {
        unsafe {
            let info = self.0;
            if !info.name.is_null() {
                drop(CString::from_raw(info.name as *mut _));
            }
            if !info.cap.is_null() {
                let mut cap = info.cap as *mut libosdp_sys::osdp_pd_cap;
                while (*cap).function_code != -1i8 as u8 {
                    cap = cap.add(1);
                }
                let len = (cap.offset_from(info.cap) + 1) as usize;
                drop(Vec::from_raw_parts(
                    info.cap as *mut libosdp_sys::osdp_pd_cap,
                    len,
                    len,
                ));
            }
            if !info.scbk.is_null() {
                drop(Box::from_raw(info.scbk as *mut [u8; 16]));
            }
        }
    }
}
