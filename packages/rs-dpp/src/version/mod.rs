use lazy_static::lazy_static;
use std::collections::HashMap;

mod protocol_version_validator;

pub const LATEST_VERSION: u32 = 1;

lazy_static! {
    pub static ref COMPATIBILITY_MAP: HashMap<u32, u32> = {
        let mut m = HashMap::new();
        m.insert(1, 1);
        m
    };
}

pub use protocol_version_validator::ProtocolVersionValidator;