//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

//! OSDP specification defines end-point devices as PDs. These devices are
//! responsible for controlling various hardware peripherals (such as LEDs,
//! buzzers, Displays, GPIOs, etc.,) and exposing them in a portable manner to
//! the controlling counter-part, the CP.
//!
//! PD receives commands from the CP and also generates events for activity that
//! happens on the PD itself (such as card read, key press, etc.,) snd sends it
//! to the CP.

use crate::{
    Box, Channel, OsdpCommand, OsdpError, OsdpEvent, OsdpFileOps, PdCapability, PdInfo,
    PdInfoBuilder,
};
use core::ffi::c_void;
use log::{debug, error, info, warn};

type Result<T> = core::result::Result<T, OsdpError>;
type CommandCallback =
    unsafe extern "C" fn(data: *mut c_void, event: *mut libosdp_sys::osdp_cmd) -> i32;

unsafe extern "C" fn log_handler(
    log_level: ::core::ffi::c_int,
    _file: *const ::core::ffi::c_char,
    _line: ::core::ffi::c_ulong,
    msg: *const ::core::ffi::c_char,
) {
    let msg = crate::cstr_to_string(msg);
    let msg = msg.trim();
    match log_level as libosdp_sys::osdp_log_level_e {
        libosdp_sys::osdp_log_level_e_OSDP_LOG_EMERG => error!("PD: {msg}"),
        libosdp_sys::osdp_log_level_e_OSDP_LOG_ALERT => error!("PD: {msg}"),
        libosdp_sys::osdp_log_level_e_OSDP_LOG_CRIT => error!("PD: {msg}"),
        libosdp_sys::osdp_log_level_e_OSDP_LOG_ERROR => error!("PD: {msg}"),
        libosdp_sys::osdp_log_level_e_OSDP_LOG_WARNING => warn!("PD: {msg}"),
        libosdp_sys::osdp_log_level_e_OSDP_LOG_NOTICE => warn!("PD: {msg}"),
        libosdp_sys::osdp_log_level_e_OSDP_LOG_INFO => info!("PD: {msg}"),
        libosdp_sys::osdp_log_level_e_OSDP_LOG_DEBUG => debug!("PD: {msg}"),
        _ => panic!("Unknown log level"),
    };
}

extern "C" fn trampoline<F>(data: *mut c_void, cmd: *mut libosdp_sys::osdp_cmd) -> i32
where
    F: FnMut(OsdpCommand) -> i32,
{
    let cmd: OsdpCommand = unsafe { (*cmd).into() };
    let callback: &mut F = unsafe { &mut *(data as *mut F) };
    callback(cmd)
}

fn get_trampoline<F>(_closure: &F) -> CommandCallback
where
    F: FnMut(OsdpCommand) -> i32,
{
    trampoline::<F>
}

fn pd_setup(info: PdInfo) -> Result<*mut c_void> {
    let info: crate::OsdpPdInfoHandle = info.into();
    let ctx = unsafe { libosdp_sys::osdp_pd_setup(&*info) };
    if ctx.is_null() {
        Err(OsdpError::Setup)
    } else {
        Ok(ctx)
    }
}

/// OSDP Peripheral Device (PD) context
#[derive(Debug)]
pub struct PeripheralDevice {
    ctx: *mut libosdp_sys::osdp_t,
}

unsafe impl Send for PeripheralDevice {}

impl PeripheralDevice {
    /// Create a new Peripheral panel object for the PD described by the corresponding PdInfo struct.
    pub fn new(info: PdInfoBuilder, channel: Box<dyn Channel>) -> Result<Self> {
        unsafe { libosdp_sys::osdp_set_log_callback(Some(log_handler)) };
        let info = info.channel(channel.into()).build();
        Ok(Self {
            ctx: pd_setup(info)?,
        })
    }

    /// This method is used to periodically refresh the underlying LibOSDP state
    /// and must be called from the application. To meet the OSDP timing
    /// guarantees, this function must be called at least once every 50ms. This
    /// method does not block and returns early if there is nothing to be done.
    pub fn refresh(&mut self) {
        unsafe { libosdp_sys::osdp_pd_refresh(self.ctx) }
    }

    /// Set a vector of [`PdCapability`] for this PD.
    pub fn set_capabilities(&mut self, cap: &[PdCapability]) {
        let cap: Vec<libosdp_sys::osdp_pd_cap> = cap
            .iter()
            .map(|c| -> libosdp_sys::osdp_pd_cap { c.clone().into() })
            .collect();
        unsafe { libosdp_sys::osdp_pd_set_capabilities(self.ctx, cap.as_ptr()) }
    }

    /// Flush or drop any events queued in this PD (but not delivered to CP yet)
    pub fn flush_events(&mut self) {
        let _ = unsafe { libosdp_sys::osdp_pd_flush_events(self.ctx) };
    }

    /// Queue and a [`OsdpEvent`] for this PD. This will be delivered to CP in
    /// the next POLL.
    pub fn notify_event(&mut self, event: OsdpEvent) -> Result<()> {
        let rc = unsafe { libosdp_sys::osdp_pd_notify_event(self.ctx, &event.into()) };
        if rc < 0 {
            Err(OsdpError::Event)
        } else {
            Ok(())
        }
    }

    /// Set a closure that gets called when this PD receives a command from the
    /// CP.
    pub fn set_command_callback<F>(&mut self, closure: F)
    where
        F: FnMut(OsdpCommand) -> i32,
    {
        unsafe {
            let callback = get_trampoline(&closure);
            libosdp_sys::osdp_pd_set_command_callback(
                self.ctx,
                Some(callback),
                Box::into_raw(Box::new(closure)).cast(),
            )
        }
    }

    /// Check online status of a PD identified by the offset number (in PdInfo
    /// vector in [`PeripheralDevice::new`]).
    pub fn is_online(&self) -> bool {
        let mut buf: u8 = 0;
        unsafe { libosdp_sys::osdp_get_status_mask(self.ctx, &mut buf as *mut u8) };
        buf != 0
    }

    /// Check secure channel status of a PD identified by the offset number
    /// (in PdInfo vector in [`PeripheralDevice::new`]). Returns (size, offset)
    /// of the current file transfer operation.
    pub fn is_sc_active(&self) -> bool {
        let mut buf: u8 = 0;
        unsafe { libosdp_sys::osdp_get_sc_status_mask(self.ctx, &mut buf as *mut u8) };
        buf != 0
    }

    /// Get status of the ongoing file transfer of PD
    pub fn file_transfer_status(&self) -> Result<(i32, i32)> {
        let mut size: i32 = 0;
        let mut offset: i32 = 0;
        let rc = unsafe {
            libosdp_sys::osdp_get_file_tx_status(
                self.ctx,
                0,
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

    /// Register a file operations handler for PD. See [`crate::OsdpFileOps`]
    /// trait documentation for more details.
    pub fn register_file_ops(&mut self, fops: Box<dyn OsdpFileOps>) -> Result<()> {
        let mut fops: libosdp_sys::osdp_file_ops = fops.into();
        let rc = unsafe {
            libosdp_sys::osdp_file_register_ops(
                self.ctx,
                0,
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

impl Drop for PeripheralDevice {
    fn drop(&mut self) {
        unsafe { libosdp_sys::osdp_pd_teardown(self.ctx) }
    }
}
