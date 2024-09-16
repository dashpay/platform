//! Epoch Info.
//!
//! This module defines and implements the `EpochInfo` struct containing
//! information about the current epoch.
//!

use crate::platform_types::epoch_info::v0::{
    EpochInfoV0, EpochInfoV0Getters, EpochInfoV0Methods, EpochInfoV0Setters,
};
use derive_more::From;
use dpp::block::epoch::{Epoch, EpochIndex};
use dpp::ProtocolError;
use serde::{Deserialize, Serialize};

pub mod v0;

/// Info pertinent to the current epoch.
///
/// BE AWARE BEFORE YOU MODIFY THIS CODE
///
/// Please be aware epoch information is gathered with previous platform version
/// on epoch change (1st block of the epoch), despite we are switching to a new version
/// in this block. Thus, the previous version of EpochInfo might also be used for the first block.
/// A new version of EpochInfo will be used for the rest of epoch blocks
/// and first block of the next epoch.
/// This means that if we ever want to update EpochInfo, we will need to do so on a release
/// where the new fields of epoch info are not being used. Then make another version once
/// that one is activated.
#[derive(Clone, Serialize, Deserialize, Debug, From, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EpochInfo {
    /// Version 0
    V0(EpochInfoV0),
}

impl EpochInfoV0Methods for EpochInfo {
    fn is_epoch_change_but_not_genesis(&self) -> bool {
        match self {
            EpochInfo::V0(v0) => v0.is_epoch_change_but_not_genesis(),
        }
    }
    fn is_first_block_of_epoch(&self, epoch_index: EpochIndex) -> bool {
        match self {
            EpochInfo::V0(v0) => v0.is_first_block_of_epoch(epoch_index),
        }
    }
}

impl EpochInfoV0Getters for EpochInfo {
    fn current_epoch_index(&self) -> u16 {
        match self {
            EpochInfo::V0(v0) => v0.current_epoch_index(),
        }
    }

    fn previous_epoch_index(&self) -> Option<u16> {
        match self {
            EpochInfo::V0(v0) => v0.previous_epoch_index(),
        }
    }

    fn is_epoch_change(&self) -> bool {
        match self {
            EpochInfo::V0(v0) => v0.is_epoch_change(),
        }
    }
}

impl EpochInfoV0Setters for EpochInfo {
    fn set_current_epoch_index(&mut self, index: u16) {
        match self {
            EpochInfo::V0(v0) => {
                v0.set_current_epoch_index(index);
            }
        }
    }

    fn set_previous_epoch_index(&mut self, index: Option<u16>) {
        match self {
            EpochInfo::V0(v0) => {
                v0.set_previous_epoch_index(index);
            }
        }
    }

    fn set_is_epoch_change(&mut self, is_epoch_change: bool) {
        match self {
            EpochInfo::V0(v0) => {
                v0.set_is_epoch_change(is_epoch_change);
            }
        }
    }
}

impl TryFrom<&EpochInfo> for Epoch {
    type Error = ProtocolError;

    fn try_from(value: &EpochInfo) -> Result<Self, Self::Error> {
        Epoch::new(value.current_epoch_index())
    }
}
