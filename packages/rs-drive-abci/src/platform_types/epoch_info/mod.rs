//! Epoch Info.
//!
//! This module defines and implements the `EpochInfo` struct containing
//! information about the current epoch.
//!

use crate::platform_types::epoch_info::v0::{
    EpochInfoV0, EpochInfoV0Getters, EpochInfoV0Methods, EpochInfoV0Setters,
};
use derive_more::From;
use dpp::block::epoch::Epoch;
use dpp::ProtocolError;
use serde::{Deserialize, Serialize};

pub mod v0;

/// Info pertinent to the current epoch.
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
