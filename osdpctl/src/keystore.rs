use std::fmt::Write;
use std::path::PathBuf;
use anyhow::Context;
use rand::Rng;

type Result<T> = anyhow::Result<T, anyhow::Error>;

fn vec_to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyStore {
    path: PathBuf,
    pub key: [u8; 16],
}

impl KeyStore {
    pub fn create(path: PathBuf, key: &str) -> Result<Self> {
        let key = KeyStore::str_to_key(key)?;
        std::fs::write(&path, key)
            .expect("Unable to write to keystore");
        Ok(Self { path, key })
    }

    pub fn _new(path: PathBuf) -> Result<Self> {
        let key = KeyStore::_random_key();
        let key_str = KeyStore::key_to_str(&key);
        std::fs::write(&path, key_str)
            .expect("Unable to write to keystore");
        Ok(Self { path, key })
    }

    pub fn _random_key() -> [u8; 16] {
        let mut key = [0u8; 16];
        rand::thread_rng().fill(&mut key);
        key
    }

    pub fn decode_hex(s: &str) -> anyhow::Result<Vec<u8>, std::num::ParseIntError> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect()
    }

    pub fn str_to_key(s: &str) -> Result<[u8; 16]> {
        let key = KeyStore::decode_hex(s)?;
        Ok(vec_to_array::<u8, 16>(key))
    }

    pub fn key_to_str(key: &[u8; 16]) -> String {
        let mut s = String::with_capacity(key.len() * 2);
        for b in key {
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }

    pub fn load(&self) -> Result<[u8; 16]> {
        let s = std::fs::read_to_string(&self.path)
            .context(format!("keystore {} not found", self.path.display()))?;
        KeyStore::str_to_key(&s)
    }

    pub fn store(&mut self, key: [u8; 16]) -> Result<()> {
        std::fs::write(&self.path, KeyStore::key_to_str(&key))
            .expect("Unable to write to keystore");
        self.key = key;
        Ok(())
    }
}
