use crate::asset_lock::reduced_asset_lock_value::v0::AssetLockValueV0;
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Bytes32;
use platform_version::version::PlatformVersion;

mod v0;

pub use v0::{AssetLockValueGettersV0, AssetLockValueSettersV0};

#[derive(Debug, Clone, Encode, Decode, PlatformSerialize, PlatformDeserialize, From, PartialEq)]
#[platform_serialize(unversioned)]
pub enum AssetLockValue {
    V0(AssetLockValueV0),
}

impl AssetLockValue {
    pub fn new(
        initial_credit_value: Credits,
        tx_out_script: Vec<u8>,
        remaining_credit_value: Credits,
        used_tags: Vec<Bytes32>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .asset_lock_versions
            .reduced_asset_lock_value
            .default_current_version
        {
            0 => Ok(AssetLockValue::V0(AssetLockValueV0 {
                initial_credit_value,
                tx_out_script,
                remaining_credit_value,
                used_tags,
            })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ReducedAssetLockValue::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl AssetLockValueGettersV0 for AssetLockValue {
    fn initial_credit_value(&self) -> Credits {
        match self {
            AssetLockValue::V0(v0) => v0.initial_credit_value,
        }
    }

    fn tx_out_script(&self) -> &Vec<u8> {
        match self {
            AssetLockValue::V0(v0) => &v0.tx_out_script,
        }
    }

    fn tx_out_script_owned(self) -> Vec<u8> {
        match self {
            AssetLockValue::V0(v0) => v0.tx_out_script,
        }
    }

    fn remaining_credit_value(&self) -> Credits {
        match self {
            AssetLockValue::V0(v0) => v0.remaining_credit_value,
        }
    }

    fn used_tags_ref(&self) -> &Vec<Bytes32> {
        match self {
            AssetLockValue::V0(v0) => &v0.used_tags,
        }
    }
}

impl AssetLockValueSettersV0 for AssetLockValue {
    fn set_initial_credit_value(&mut self, value: Credits) {
        match self {
            AssetLockValue::V0(v0) => v0.initial_credit_value = value,
        }
    }

    fn set_tx_out_script(&mut self, value: Vec<u8>) {
        match self {
            AssetLockValue::V0(v0) => v0.tx_out_script = value,
        }
    }

    fn set_remaining_credit_value(&mut self, value: Credits) {
        match self {
            AssetLockValue::V0(v0) => v0.remaining_credit_value = value,
        }
    }

    fn set_used_tags(&mut self, tags: Vec<Bytes32>) {
        match self {
            AssetLockValue::V0(v0) => v0.used_tags = tags,
        }
    }

    fn add_used_tag(&mut self, tag: Bytes32) {
        match self {
            AssetLockValue::V0(v0) => v0.used_tags.push(tag),
        }
    }
}
