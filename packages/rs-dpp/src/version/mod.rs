// use std::collections::HashMap;
//
// use lazy_static::lazy_static;
//
// mod protocol_version;
// pub use protocol_version::*;
// pub mod dpp_versions;
// pub mod drive_abci_versions;
// pub mod drive_versions;
// mod v0;
// #[cfg(feature = "validation")]
// mod validation;
//
// pub const LATEST_VERSION: u32 = 1;
//
// lazy_static! {
//     pub static ref COMPATIBILITY_MAP: HashMap<u32, u32> = {
//         let mut m = HashMap::new();
//         m.insert(1, 1);
//         m
//     };
// }

pub use platform_version::version::*;
