//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

//! The Control Panel (CP) is responsible to connecting to and managing multiple Peripheral Devices
//! (PD) on the OSDP bus. It can send commands to and receive events from PDs.

use crate::{
    file::OsdpFileOps, Channel, OsdpCommand, OsdpError, OsdpEvent, OsdpFlag, PdCapability, PdId,
    PdInfoBuilder,
};
use alloc::{boxed::Box, vec::Vec};
use core::ffi::c_void;
#[cfg(feature = "defmt-03")]
use defmt::{debug, error, info, warn};
#[cfg(all(feature = "log", not(feature = "defmt-03")))]
use log::{debug, error, info, warn};

type Result<T> = core::result::Result<T, OsdpError>;

unsafe extern "C" fn log_handler(
    _log_level: ::core::ffi::c_int,
    _file: *const ::core::ffi::c_char,
    _line: ::core::ffi::c_ulong,
    _msg: *const ::core::ffi::c_char,
) {
    #[cfg(any(feature = "log", feature = "defmt-03"))]
    {
        let msg = crate::cstr_to_string(_msg);
        let msg = msg.trim();
        match _log_level as libosdp_sys::osdp_log_level_e {
            libosdp_sys::osdp_log_level_e_OSDP_LOG_EMERG => error!("CP: {}", msg),
            libosdp_sys::osdp_log_level_e_OSDP_LOG_ALERT => error!("CP: {}", msg),
            libosdp_sys::osdp_log_level_e_OSDP_LOG_CRIT => error!("CP: {}", msg),
            libosdp_sys::osdp_log_level_e_OSDP_LOG_ERROR => error!("CP: {}", msg),
            libosdp_sys::osdp_log_level_e_OSDP_LOG_WARNING => warn!("CP: {}", msg),
            libosdp_sys::osdp_log_level_e_OSDP_LOG_NOTICE => warn!("CP: {}", msg),
            libosdp_sys::osdp_log_level_e_OSDP_LOG_INFO => info!("CP: {}", msg),
            libosdp_sys::osdp_log_level_e_OSDP_LOG_DEBUG => debug!("CP: {}", msg),
            _ => panic!("Unknown log level"),
        };
    }
}

extern "C" fn trampoline<F>(data: *mut c_void, pd: i32, event: *mut libosdp_sys::osdp_event) -> i32
where
    F: FnMut(i32, OsdpEvent) -> i32,
{
    let event: OsdpEvent = unsafe { (*event).into() };
    let callback: &mut F = unsafe { &mut *(data as *mut F) };
    callback(pd, event)
}

type EventCallback =
    unsafe extern "C" fn(data: *mut c_void, pd: i32, event: *mut libosdp_sys::osdp_event) -> i32;

fn get_trampoline<F>(_closure: &F) -> EventCallback
where
    F: FnMut(i32, OsdpEvent) -> i32,
{
    trampoline::<F>
}

fn cp_setup(info: Vec<crate::OsdpPdInfoHandle>) -> Result<*mut c_void> {
    let ctx = unsafe { libosdp_sys::osdp_cp_setup(info.len() as i32, info.as_ptr() as *const _) };
    if ctx.is_null() {
        Err(OsdpError::Setup)
    } else {
        Ok(ctx)
    }
}

/// Builder for creating a new `ControlPanel`.
#[derive(Debug, Default)]
pub struct ControlPanelBuilder {
    channel_pds: Vec<(Box<dyn Channel>, Vec<PdInfoBuilder>)>,
}

impl ControlPanelBuilder {
    /// Create a new instance of [`ControlPanelBuilder`].
    pub const fn new() -> Self {
        Self {
            channel_pds: Vec::new(),
        }
    }

    /// Add a new PDs and their shared channel to the CP.
    pub fn add_channel(mut self, channel: Box<dyn Channel>, pd_info: Vec<PdInfoBuilder>) -> Self {
        self.channel_pds.push((channel, pd_info));
        self
    }

    /// Build the [`ControlPanel`] instance.
    pub fn build(self) -> Result<ControlPanel> {
        if self.channel_pds.len() > 126 {
            return Err(OsdpError::PdInfo("max PD count exceeded"));
        }
        let info: Vec<crate::OsdpPdInfoHandle> = self
            .channel_pds
            .into_iter()
            .map(|(channel, pd_info)| {
                let channel: libosdp_sys::osdp_channel = channel.into();
                pd_info
                    .into_iter()
                    .map(move |pd| pd.channel(channel).build().into())
            })
            .flatten()
            .collect();
        unsafe { libosdp_sys::osdp_set_log_callback(Some(log_handler)) };
        Ok(ControlPanel {
            ctx: cp_setup(info)?,
        })
    }
}

/// OSDP CP device context.
#[derive(Debug)]
pub struct ControlPanel {
    ctx: *mut core::ffi::c_void,
}

unsafe impl Send for ControlPanel {}

impl ControlPanel {
    /// The application must call this method periodically to refresh the
    /// underlying LibOSDP state. To meet the OSDP timing guarantees, this
    /// function must be called at least once every 50ms. This method does not
    /// block and returns early if there is nothing to be done.
    pub fn refresh(&mut self) {
        unsafe { libosdp_sys::osdp_cp_refresh(self.ctx) }
    }

    /// Send [`OsdpCommand`] to a PD identified by the offset number (in PdInfo
    /// vector in [`ControlPanel::new`]).
    pub fn send_command(&mut self, pd: i32, cmd: OsdpCommand) -> Result<()> {
        let rc = unsafe { libosdp_sys::osdp_cp_send_command(self.ctx, pd, &cmd.into()) };
        if rc < 0 {
            Err(OsdpError::Command)
        } else {
            Ok(())
        }
    }

    /// Set a closure that gets called when a PD sends an event to this CP.
    pub fn set_event_callback<F>(&mut self, closure: F)
    where
        F: FnMut(i32, OsdpEvent) -> i32,
    {
        unsafe {
            let callback = get_trampoline(&closure);
            libosdp_sys::osdp_cp_set_event_callback(
                self.ctx,
                Some(callback),
                Box::into_raw(Box::new(closure)).cast(),
            );
        }
    }

    /// Get the [`PdId`] from a PD identified by the offset number (in PdInfo
    /// vector in [`ControlPanel::new`]).
    pub fn get_pd_id(&self, pd: i32) -> Result<PdId> {
        let mut pd_id: libosdp_sys::osdp_pd_id =
            unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        let rc = unsafe { libosdp_sys::osdp_cp_get_pd_id(self.ctx, pd, &mut pd_id) };
        if rc < 0 {
            Err(OsdpError::Query("PdId"))
        } else {
            Ok(pd_id.into())
        }
    }

    /// Get the [`PdCapability`] from a PD identified by the offset number (in
    /// PdInfo vector in [`ControlPanel::new`]).
    pub fn get_capability(&self, pd: i32, cap: PdCapability) -> Result<PdCapability> {
        let mut cap = cap.into();
        let rc = unsafe { libosdp_sys::osdp_cp_get_capability(self.ctx, pd, &mut cap) };
        if rc < 0 {
            Err(OsdpError::Query("capability"))
        } else {
            Ok(cap.into())
        }
    }

    /// Set [`OsdpFlag`] for a PD identified by the offset number (in PdInfo
    /// vector in [`ControlPanel::new`]).
    pub fn set_flag(&mut self, pd: i32, flags: OsdpFlag, value: bool) {
        let rc = unsafe { libosdp_sys::osdp_cp_modify_flag(self.ctx, pd, flags.bits(), value) };
        if rc < 0 {
            // OsdpFlag should guarantee that we never fail here. If we did,
            // it's probably best to panic here.
            panic!("osdp_cp_modify_flag failed!")
        }
    }

    /// Check online status of a PD identified by the offset number (in PdInfo
    /// vector in [`ControlPanel::new`]).
    pub fn is_online(&self, pd: i32) -> bool {
        let mut buf: [u8; 16] = [0; 16];
        unsafe { libosdp_sys::osdp_get_status_mask(self.ctx, &mut buf as *mut u8) };
        let pos = pd / 8;
        let idx = pd % 8;
        buf[pos as usize] & (1 << idx) != 0
    }

    /// Check secure channel status of a PD identified by the offset number
    /// (in PdInfo vector in [`ControlPanel::new`]).
    pub fn is_sc_active(&self, pd: i32) -> bool {
        let mut buf: [u8; 16] = [0; 16];
        unsafe { libosdp_sys::osdp_get_sc_status_mask(self.ctx, &mut buf as *mut u8) };
        let pos = pd / 8;
        let idx = pd % 8;
        buf[pos as usize] & (1 << idx) != 0
    }

    /// Get status of the ongoing file transfer of a PD, identified by the
    /// offset number (in PdInfo vector in [`ControlPanel::new`]). Returns
    /// (size, offset) of the current file transfer operation.
    pub fn file_transfer_status(&self, pd: i32) -> Result<(i32, i32)> {
        let mut size: i32 = 0;
        let mut offset: i32 = 0;
        let rc = unsafe {
            libosdp_sys::osdp_get_file_tx_status(
                self.ctx,
                pd,
                &mut size as *mut i32,
                &mut offset as *mut i32,
            )
        };
        if rc < 0 {
            Err(OsdpError::FileTransfer("Not not in progress"))
        } else {
            Ok((size, offset))
        }
    }

    /// Register a file operations handler for a PD. See [`crate::OsdpFileOps`]
    /// trait documentation for more details.
    pub fn register_file_ops(&mut self, pd: i32, fops: Box<dyn OsdpFileOps>) -> Result<()> {
        let mut fops: libosdp_sys::osdp_file_ops = fops.into();
        let rc = unsafe {
            libosdp_sys::osdp_file_register_ops(
                self.ctx,
                pd,
                &mut fops as *mut libosdp_sys::osdp_file_ops,
            )
        };
        if rc < 0 {
            Err(OsdpError::FileTransfer("ops register"))
        } else {
            Ok(())
        }
    }
}

impl Drop for ControlPanel {
    fn drop(&mut self) {
        unsafe { libosdp_sys::osdp_cp_teardown(self.ctx) }
    }
}
