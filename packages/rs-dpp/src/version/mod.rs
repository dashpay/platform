use lazy_static::lazy_static;
use std::collections::HashMap;

mod protocol_version_validator;

pub const LATEST_VERSION: u64 = 1;

lazy_static! {
    pub static ref COMPATIBILITY_MAP: HashMap<u64, u64> = {
        let mut m = HashMap::new();
        m.insert(1, 1);
        m
    };
}

pub use protocol_version_validator::ProtocolVersionValidator;
