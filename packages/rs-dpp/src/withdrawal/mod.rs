use bincode::{Decode, Encode};
use dashcore::{ScriptBuf, TxOut};
use dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::ASSET_UNLOCK_TX_SIZE;
use dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo};
use serde_repr::{Deserialize_repr, Serialize_repr};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_version::version::PlatformVersion;
use crate::document::{Document, DocumentV0Getters};
use crate::identity::convert_credits_to_duffs;
use crate::ProtocolError;
use crate::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

#[repr(u8)]
#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Encode, Decode, Default,
)]
pub enum Pooling {
    #[default]
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

/// Transaction index type
pub type WithdrawalTransactionIndex = u64;

/// Simple type alias for withdrawal transaction with it's index
pub type WithdrawalTransactionIndexAndBytes = (WithdrawalTransactionIndex, Vec<u8>);

impl Document {
    pub fn try_into_asset_unlock_base_transaction_info(&self, transaction_index: WithdrawalTransactionIndex, platform_version: &PlatformVersion) -> Result<AssetUnlockBaseTransactionInfo, ProtocolError> {
        match platform_version
            .dpp
            .document_versions.document_method_versions
            .try_into_asset_unlock_base_transaction_info
        {
            0 => self.try_into_asset_unlock_base_transaction_info_v0(
                transaction_index
            ),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }

    fn try_into_asset_unlock_base_transaction_info_v0(&self, transaction_index: WithdrawalTransactionIndex) -> Result<AssetUnlockBaseTransactionInfo, ProtocolError> {
        let properties = self.properties();

        let output_script_bytes = properties.get_bytes(withdrawal::properties::OUTPUT_SCRIPT)?;

        let amount = properties
            .get_integer(withdrawal::properties::AMOUNT)?;

        let core_fee_per_byte: u32 = properties
            .get_integer(withdrawal::properties::CORE_FEE_PER_BYTE)?;

        let output_script = ScriptBuf::from_bytes(output_script_bytes);

        let tx_out = TxOut {
            value: convert_credits_to_duffs(amount)?,
            script_pubkey: output_script,
        };

        Ok(AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![tx_out],
            base_payload: AssetUnlockBasePayload {
                version: 1,
                index: transaction_index,
                fee: ASSET_UNLOCK_TX_SIZE as u32 * core_fee_per_byte,
            },
        })
    }
}