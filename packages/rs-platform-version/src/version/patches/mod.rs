use crate::version::{PlatformVersion, ProtocolVersion};
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;

/// Patch function signature. It uses to dynamically modify platform version
pub type PatchFn = fn(PlatformVersion) -> PlatformVersion;

type HeightToPatchRanges = BTreeMap<u64, PatchFn>;

/// Patch function per height, per protocol version
pub static PATCHES: Lazy<RwLock<HashMap<ProtocolVersion, HeightToPatchRanges>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
