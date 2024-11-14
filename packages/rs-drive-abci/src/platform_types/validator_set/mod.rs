use crate::platform_types::validator_set::v0::ValidatorSetMethodsV0;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

pub use dpp::core_types::validator_set::*;

/// v0
pub mod v0;

pub(crate) trait ValidatorSetExt {
    fn to_update(&self) -> ValidatorSetUpdate;
    #[allow(unused)]
    fn to_update_owned(self) -> ValidatorSetUpdate;
}

impl ValidatorSetExt for ValidatorSet {
    fn to_update(&self) -> ValidatorSetUpdate {
        match self {
            ValidatorSet::V0(v0) => v0.to_update(),
        }
    }

    fn to_update_owned(self) -> ValidatorSetUpdate {
        match self {
            ValidatorSet::V0(v0) => v0.to_update_owned(),
        }
    }
}
