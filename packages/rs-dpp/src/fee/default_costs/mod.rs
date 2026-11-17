//! Fee costs
//!
//! Fee costs for Known Platform operations
//!

use crate::block::epoch::EpochIndex;
use platform_version::version::fee::{
    FeeVersion, FeeVersionFieldsBeforeVersion4, FeeVersionNumber,
};
use std::collections::BTreeMap;

pub mod constants;

pub type CachedEpochIndexFeeVersions = BTreeMap<EpochIndex, &'static FeeVersion>;
pub type EpochIndexFeeVersionsForStorage = BTreeMap<EpochIndex, FeeVersionNumber>;

// This is type only meant for deserialization because of an issue
// The issue was that the platform state was stored with FeeVersions in it before version 1.4
// When we would add new fields we would be unable to deserialize
// This FeeProcessingVersionFieldsBeforeVersion4 is how things were before version 1.4 was released
pub type CachedEpochIndexFeeVersionsFieldsBeforeVersion4 =
    BTreeMap<EpochIndex, FeeVersionFieldsBeforeVersion4>;
