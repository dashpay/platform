use crate::asset_lock::reduced_asset_lock_value::v0::{
    AssetLockValueV0,
};
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_version::version::PlatformVersion;

mod v0;

pub use v0::{AssetLockValueGettersV0, AssetLockValueSettersV0};

#[derive(
    Debug, Clone, Copy, Encode, Decode, PlatformSerialize, PlatformDeserialize, From, PartialEq,
)]
#[platform_serialize(unversioned)]
pub enum AssetLockValue {
    V0(AssetLockValueV0),
}

impl AssetLockValue {
    pub fn new(
        initial_credit_value: Credits,
        remaining_credit_value: Credits,
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
                remaining_credit_value,
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

    fn remaining_credit_value(&self) -> Credits {
        match self {
            AssetLockValue::V0(v0) => v0.remaining_credit_value,
        }
    }
}

impl AssetLockValueSettersV0 for AssetLockValue {
    fn set_initial_credit_value(&mut self, value: Credits) {
        match self {
            AssetLockValue::V0(v0) => v0.initial_credit_value = value,
        }
    }

    fn set_remaining_credit_value(&mut self, value: Credits) {
        match self {
            AssetLockValue::V0(v0) => v0.remaining_credit_value = value,
        }
    }
}
