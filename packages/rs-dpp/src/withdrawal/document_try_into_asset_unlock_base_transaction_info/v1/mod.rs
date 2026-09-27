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
use platform_version::version::PlatformVersion;

impl Document {
    pub(super) fn try_into_asset_unlock_base_transaction_info_v1(
        &self,
        transaction_index: WithdrawalTransactionIndex,
        platform_version: &PlatformVersion,
    ) -> Result<AssetUnlockBaseTransactionInfo, ProtocolError> {
        let properties = self.properties();
        let output_script_bytes = properties.get_bytes(withdrawal::properties::OUTPUT_SCRIPT)?;
        let withdrawal_amount = properties.get_integer(withdrawal::properties::AMOUNT)?;
        let core_fee_per_byte: u32 =
            properties.get_integer(withdrawal::properties::CORE_FEE_PER_BYTE)?;

        let max_core_fee_per_byte = platform_version
            .system_limits
            .max_core_fee_per_byte
            .ok_or_else(|| {
                ProtocolError::Generic(
                    "asset unlock conversion v1 requires a Core fee limit".to_string(),
                )
            })?;
        let effective_core_fee_per_byte = core_fee_per_byte.min(max_core_fee_per_byte);
        let requested_fee = (ASSET_UNLOCK_TX_SIZE as u64)
            .checked_mul(effective_core_fee_per_byte as u64)
            .ok_or_else(|| ProtocolError::Generic("asset unlock fee overflow".to_string()))?;
        let withdrawal_amount_duffs = convert_credits_to_duffs(withdrawal_amount)?;
        let output_script = ScriptBuf::from_bytes(output_script_bytes);

        // An unstamped document was queued before this accounting rule activated. Its requested
        // fee can exceed both the new cap and the amount it reserved. Bound that fee by the new
        // cap and by the value available above Core's dust threshold so the old queue drains
        // without taking any additional value from the Core credit pool.
        //
        // A legacy amount at or below the dust threshold (reachable under the 190-duff floor of
        // protocol versions 11 and below) saturates to a zero fee instead of failing. This
        // conversion runs inside a per-block event, where an error halts block production on
        // every node, whereas a dust-sized asset unlock merely stays unrelayed, exactly as it
        // did before this rule.
        let fee = if self.contract_version().is_none() {
            let available_above_dust =
                withdrawal_amount_duffs.saturating_sub(output_script.dust_value().to_sat());
            requested_fee.min(available_above_dust)
        } else {
            requested_fee
        };

        let fee: u32 = fee
            .try_into()
            .map_err(|_| ProtocolError::Generic("asset unlock fee overflow".to_string()))?;
        let output_amount = withdrawal_amount_duffs
            .checked_sub(fee as u64)
            .ok_or_else(|| {
                ProtocolError::Generic(
                    "withdrawal amount does not cover the asset unlock fee".to_string(),
                )
            })?;

        Ok(AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![TxOut {
                value: output_amount,
                script_pubkey: output_script,
            }],
            base_payload: AssetUnlockBasePayload {
                version: 1,
                index: transaction_index,
                fee,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentV0;
    use crate::identity::core_script::CoreScript;
    use crate::platform_value::platform_value;
    use crate::system_data_contracts::withdrawals_contract;
    use crate::version::PlatformVersion;
    use crate::withdrawal::Pooling;

    fn withdrawal_document(contract_version: Option<u32>, amount: u64) -> Document {
        DocumentV0 {
            contract_version,
            id: Default::default(),
            owner_id: Default::default(),
            properties: platform_value!({
                withdrawal::properties::AMOUNT: amount,
                withdrawal::properties::CORE_FEE_PER_BYTE: 6_765u32,
                withdrawal::properties::POOLING: Pooling::Never,
                withdrawal::properties::OUTPUT_SCRIPT: CoreScript::new_p2pkh([1u8; 20]).to_bytes(),
                withdrawal::properties::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
            })
            .into_btree_string_map()
            .expect("withdrawal properties"),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into()
    }

    #[test]
    fn stamped_withdrawal_reserves_output_and_core_fee_from_one_amount() {
        let amount = 2_000_000_000u64;
        let tx = withdrawal_document(Some(1), amount)
            .try_into_asset_unlock_base_transaction_info(
                1,
                PlatformVersion::get(14).expect("platform version 14"),
            )
            .expect("asset unlock info");

        assert_eq!(
            tx.output[0].value + tx.base_payload.fee as u64,
            convert_credits_to_duffs(amount).expect("duffs")
        );
        assert_eq!(tx.base_payload.fee, 1_285_350);
    }

    #[test]
    fn unstamped_queued_withdrawal_is_bounded_by_its_reserved_amount() {
        let amount = 1_000_000u64;
        let tx = withdrawal_document(None, amount)
            .try_into_asset_unlock_base_transaction_info(
                1,
                PlatformVersion::get(14).expect("platform version 14"),
            )
            .expect("asset unlock info");

        assert_eq!(
            tx.output[0].value + tx.base_payload.fee as u64,
            convert_credits_to_duffs(amount).expect("duffs")
        );
        assert!(tx.output[0].value >= tx.output[0].script_pubkey.dust_value().to_sat());
        assert!(tx.base_payload.fee < 1_285_350);
    }

    #[test]
    fn unstamped_queued_withdrawal_uses_the_v14_fee_cap() {
        let amount = 2_000_000_000u64;
        let mut document = withdrawal_document(None, amount);
        document.properties_mut().insert(
            withdrawal::properties::CORE_FEE_PER_BYTE.to_string(),
            10_946u32.into(),
        );
        let tx = document
            .try_into_asset_unlock_base_transaction_info(
                1,
                PlatformVersion::get(14).expect("platform version 14"),
            )
            .expect("asset unlock info");

        assert_eq!(tx.base_payload.fee, 1_285_350);
        assert_eq!(
            tx.output[0].value + tx.base_payload.fee as u64,
            convert_credits_to_duffs(amount).expect("duffs")
        );
    }

    #[test]
    fn protocol_version_13_keeps_the_core_fee_outside_the_amount() {
        let amount = 2_000_000_000u64;
        let tx = withdrawal_document(None, amount)
            .try_into_asset_unlock_base_transaction_info(
                1,
                PlatformVersion::get(13).expect("platform version 13"),
            )
            .expect("asset unlock info");

        // Before v14 the output carried the whole amount and Core drew the fee on top of it.
        assert_eq!(
            tx.output[0].value,
            convert_credits_to_duffs(amount).expect("duffs")
        );
        assert_eq!(tx.base_payload.fee, 1_285_350);
    }

    #[test]
    fn unstamped_queued_withdrawal_at_or_below_dust_pays_no_core_fee() {
        // 500 duffs: below the 546-duff P2PKH dust threshold, legal under the pre-v12 floor.
        let amount = 500_000u64;
        let tx = withdrawal_document(None, amount)
            .try_into_asset_unlock_base_transaction_info(
                1,
                PlatformVersion::get(14).expect("platform version 14"),
            )
            .expect("a legacy dust amount must convert rather than abort the block");

        assert_eq!(tx.base_payload.fee, 0);
        assert_eq!(
            tx.output[0].value,
            convert_credits_to_duffs(amount).expect("duffs")
        );
    }
}
