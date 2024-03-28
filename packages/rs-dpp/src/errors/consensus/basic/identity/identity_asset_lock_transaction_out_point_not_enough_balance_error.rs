use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode::{Decode, Encode};
use dashcore::Txid;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Asset lock transaction {transaction_id} output {output_index} only has {credits_left} credits left out of {initial_asset_lock_credits} initial credits on the asset lock but needs {credits_required} credits to start processing")]
#[platform_serialize(unversioned)]
pub struct IdentityAssetLockTransactionOutPointNotEnoughBalanceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[platform_serialize(with_serde)]
    #[bincode(with_serde)]
    transaction_id: Txid,
    output_index: usize,
    initial_asset_lock_credits: Credits,
    credits_left: Credits,
    credits_required: Credits,
}

impl IdentityAssetLockTransactionOutPointNotEnoughBalanceError {
    pub fn new(
        transaction_id: Txid,
        output_index: usize,
        initial_asset_lock_credits: Credits,
        credits_left: Credits,
        credits_required: Credits,
    ) -> Self {
        Self {
            transaction_id,
            output_index,
            initial_asset_lock_credits,
            credits_left,
            credits_required,
        }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }

    pub fn transaction_id(&self) -> Txid {
        self.transaction_id
    }

    pub fn initial_asset_lock_credits(&self) -> Credits {
        self.initial_asset_lock_credits
    }

    pub fn credits_left(&self) -> Credits {
        self.credits_left
    }

    pub fn credits_required(&self) -> Credits {
        self.credits_required
    }
}

impl From<IdentityAssetLockTransactionOutPointNotEnoughBalanceError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionOutPointNotEnoughBalanceError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockTransactionOutPointNotEnoughBalanceError(err))
    }
}
