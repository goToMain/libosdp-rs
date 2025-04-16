use std::hash::{DefaultHasher, Hash, Hasher};

pub mod unix;
pub mod serial;

pub fn str_to_channel_id(key: &str) -> i32 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let mut id: u64 = hasher.finish();
    id = (id >> 32) ^ id & 0xffffffff;
    id as i32
}
