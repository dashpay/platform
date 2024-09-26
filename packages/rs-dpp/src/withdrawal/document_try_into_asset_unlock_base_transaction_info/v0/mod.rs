use crate::document::{Document, DocumentV0Getters};
use crate::identity::convert_credits_to_duffs;
use crate::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use crate::withdrawal::WithdrawalTransactionIndex;
use crate::ProtocolError;
use dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::ASSET_UNLOCK_TX_SIZE;
use dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
    AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
};
use dashcore::{ScriptBuf, TxOut};
use platform_value::btreemap_extensions::BTreeValueMapHelper;

impl Document {
    pub(super) fn try_into_asset_unlock_base_transaction_info_v0(
        &self,
        transaction_index: WithdrawalTransactionIndex,
    ) -> Result<AssetUnlockBaseTransactionInfo, ProtocolError> {
        let properties = self.properties();

        let output_script_bytes = properties.get_bytes(withdrawal::properties::OUTPUT_SCRIPT)?;

        let amount = properties.get_integer(withdrawal::properties::AMOUNT)?;

        let core_fee_per_byte: u32 =
            properties.get_integer(withdrawal::properties::CORE_FEE_PER_BYTE)?;

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
