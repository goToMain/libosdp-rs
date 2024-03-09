//
// Copyright (c) 2023-2024 Siddharth Chandrasekaran <sidcha.dev@gmail.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}};

pub mod device;
pub mod threadbus;
pub mod memory_channel;
pub mod unix_channel;

pub fn setup() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(false)
        .format_timestamp(None)
        .init();
}

pub fn str_to_channel_id(key: &str) -> i32 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let mut id: u64 = hasher.finish();
    id = (id >> 32) ^ id & 0xffffffff;
    id as i32
}
