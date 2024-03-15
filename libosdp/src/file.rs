//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

//! OSDP provides a means to send files from CP to a Peripheral Device (PD).
//! This module adds the required components to achieve this effect.

use core::ffi::c_void;

type Result<T> = core::result::Result<T, crate::OsdpError>;

/// File operations handler trait. Any type that implements this trait can be
/// registered with [`crate::ControlPanel::register_file_ops`] or
/// [`crate::PeripheralDevice::register_file_ops`].
pub trait OsdpFileOps {
    /// Open a file, with pre-agreed File-ID [`id`]; returns the size of the
    /// file that was opened or [`crate::OsdpError::FileTransfer`].
    fn open(&mut self, id: i32, read_only: bool) -> Result<usize>;
    /// Read bytes into buffer [`buf`] from offset [`off`] of the file; returns
    /// number of bytes read or [`crate::OsdpError::FileTransfer`].
    fn offset_read(&self, buf: &mut [u8], off: u64) -> Result<usize>;
    /// Write bytes from buffer [`buf`] at offset [`off`] of the file; returns
    /// number of bytes written or [`crate::OsdpError::FileTransfer`].
    fn offset_write(&self, buf: &[u8], off: u64) -> Result<usize>;
    /// Close the currently open file; returns [`crate::OsdpError::FileTransfer`]
    /// if close failed.
    fn close(&mut self) -> Result<()>;
}

unsafe extern "C" fn file_open(data: *mut c_void, file_id: i32, size: *mut i32) -> i32 {
    let ctx: *mut Box<dyn OsdpFileOps> = data as *mut _;
    let ctx = ctx.as_mut().unwrap();
    let read_only = *size == 0;
    match ctx.open(file_id, read_only) {
        Ok(file_size) => {
            if read_only {
                *size = file_size as i32;
            }
            0
        }
        Err(e) => {
            log::error!("open: {:?}", e);
            -1
        }
    }
}

unsafe extern "C" fn file_read(data: *mut c_void, buf: *mut c_void, size: i32, offset: i32) -> i32 {
    let ctx: *mut Box<dyn OsdpFileOps> = data as *mut _;
    let ctx = ctx.as_ref().unwrap();
    let mut read_buf = vec![0u8; size as usize];
    let len = match ctx.offset_read(&mut read_buf, offset as u64) {
        Ok(len) => len as i32,
        Err(e) => {
            log::error!("file_read: {:?}", e);
            -1
        }
    };
    std::ptr::copy_nonoverlapping(read_buf.as_mut_ptr(), buf as *mut u8, len as usize);
    len
}

unsafe extern "C" fn file_write(
    data: *mut c_void,
    buf: *const c_void,
    size: i32,
    offset: i32,
) -> i32 {
    let ctx: *mut Box<dyn OsdpFileOps> = data as *mut _;
    let ctx = ctx.as_ref().unwrap();
    let mut write_buf = vec![0u8; size as usize];
    std::ptr::copy_nonoverlapping(buf as *mut u8, write_buf.as_mut_ptr(), size as usize);
    match ctx.offset_write(&write_buf, offset as u64) {
        Ok(len) => len as i32,
        Err(e) => {
            log::error!("file_write: {:?}", e);
            -1
        }
    }
}

unsafe extern "C" fn file_close(data: *mut c_void) -> i32 {
    let ctx: *mut Box<dyn OsdpFileOps> = data as *mut _;
    let ctx = ctx.as_mut().unwrap();
    match ctx.close() {
        Ok(_) => 0,
        Err(e) => {
            log::error!("file_close: {:?}", e);
            -1
        }
    }
}

impl From<Box<dyn OsdpFileOps>> for libosdp_sys::osdp_file_ops {
    fn from(value: Box<dyn OsdpFileOps>) -> Self {
        let data = Box::into_raw(Box::new(value));
        libosdp_sys::osdp_file_ops {
            arg: data as *mut _ as *mut c_void,
            open: Some(file_open),
            read: Some(file_read),
            write: Some(file_write),
            close: Some(file_close),
        }
    }
}
