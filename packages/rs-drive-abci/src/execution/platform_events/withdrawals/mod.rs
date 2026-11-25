pub(in crate::execution) mod append_signatures_and_broadcast_withdrawal_transactions;
pub(in crate::execution) mod build_untied_withdrawal_transactions_from_documents;
pub(in crate::execution) mod calculate_core_anchored_withdrawal_limit;
pub(in crate::execution) mod cleanup_expired_locks_of_withdrawal_amounts;
pub(in crate::execution) mod dequeue_and_build_unsigned_withdrawal_transactions;
pub(in crate::execution) mod fetch_transactions_block_inclusion_status;
pub(in crate::execution) mod has_pending_withdrawal_work;
pub(in crate::execution) mod pool_withdrawals_into_transactions_queue;
pub(in crate::execution) mod rebroadcast_expired_withdrawal_documents;
pub(in crate::execution) mod record_credit_inflows_for_withdrawals;
pub(in crate::execution) mod record_total_credits_history_for_withdrawals;
pub(in crate::execution) mod scan_core_blocks_for_withdrawals;
pub(in crate::execution) mod update_broadcasted_withdrawal_statuses;

use crate::error::Error;
use dpp::dashcore::ScriptBuf;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::convert_credits_to_duffs;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::withdrawal::core_dust_threshold_duffs;

/// Whether the whole withdrawal amount is below the dust threshold Core's mempool applies
/// to the withdrawal's output script, so no asset unlock built from this document can
/// ever be relayed.
///
/// `rebroadcast_expired_withdrawal_documents` 2 marks an expired withdrawal FAILED, and
/// stops re-signing it, on this predicate. `build_untied_withdrawal_transactions_from_documents`
/// 1 keeps such a withdrawal at asset unlock payload version 1 on it: Platform must keep
/// re-signing a version 2 withdrawal until it is mined, as an InstantSend lock may already
/// rely on it, so FAILED may only ever reach a version 1 withdrawal, which Core never
/// InstantSend-locks. Both read this one predicate, so at the same dust relay fee they agree.
///
/// Protocol version 14 selects `rebroadcast_expired_withdrawal_documents` 2, so an edit here
/// changes a shipped generation. A withdrawal can be pooled under one protocol version and
/// expire under a later one, so a protocol version that raises `core_dust_relay_fee_per_kb`,
/// or needs another predicate, must keep FAILED reaching only version 1 withdrawals,
/// including those pooled before it.
fn withdrawal_amount_is_core_dust(
    document: &Document,
    dust_relay_fee_per_kb: u64,
) -> Result<bool, Error> {
    let amount: u64 = document
        .properties()
        .get_integer(withdrawal::properties::AMOUNT)?;

    let output_script_bytes = document
        .properties()
        .get_bytes(withdrawal::properties::OUTPUT_SCRIPT)?;

    let amount_duffs = convert_credits_to_duffs(amount).map_err(Error::Protocol)?;

    let output_script = ScriptBuf::from_bytes(output_script_bytes);

    Ok(amount_duffs < core_dust_threshold_duffs(&output_script, dust_relay_fee_per_kb))
}

#[cfg(test)]
mod tests {
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{PubkeyHash, ScriptBuf, ScriptHash};
    use dpp::identity::convert_credits_to_duffs;
    use dpp::version::PLATFORM_VERSIONS;
    use dpp::withdrawal::core_dust_threshold_duffs;

    /// FAILED may only ever reach a withdrawal of asset unlock payload version 1 (see
    /// `withdrawal_amount_is_core_dust`). A withdrawal pooled as version 2 under one protocol
    /// version can expire under a later one, so a later dust relay fee must never be higher,
    /// and a withdrawal of the minimum amount must never be Core dust, for the output scripts a
    /// withdrawal can pay (P2PKH and P2SH).
    #[test]
    fn should_never_raise_the_dust_relay_fee_nor_make_the_minimum_withdrawal_dust() {
        let output_scripts = [
            ("P2PKH", ScriptBuf::new_p2pkh(&PubkeyHash::all_zeros())),
            ("P2SH", ScriptBuf::new_p2sh(&ScriptHash::all_zeros())),
        ];
        let mut previous_dust_relay_fee = None;
        for platform_version in PLATFORM_VERSIONS {
            let protocol_version = platform_version.protocol_version;
            let Some(dust_relay_fee_per_kb) =
                platform_version.system_limits.core_dust_relay_fee_per_kb
            else {
                continue;
            };
            if let Some((previous_version, previous_fee)) = previous_dust_relay_fee {
                assert!(
                    dust_relay_fee_per_kb <= previous_fee,
                    "protocol version {protocol_version} raises the dust relay fee of protocol \
                     version {previous_version} from {previous_fee} to {dust_relay_fee_per_kb} \
                     duffs/kB"
                );
            }
            let min_withdrawal_duffs =
                convert_credits_to_duffs(platform_version.system_limits.min_withdrawal_amount)
                    .expect("expected the minimum withdrawal in duffs");
            for (name, output_script) in &output_scripts {
                let dust_threshold =
                    core_dust_threshold_duffs(output_script, dust_relay_fee_per_kb);
                assert!(
                    min_withdrawal_duffs >= dust_threshold,
                    "protocol version {protocol_version}: the minimum withdrawal of \
                     {min_withdrawal_duffs} duffs is below the {name} dust threshold of \
                     {dust_threshold} duffs"
                );
            }
            previous_dust_relay_fee = Some((protocol_version, dust_relay_fee_per_kb));
        }
        assert!(
            previous_dust_relay_fee.is_some(),
            "some protocol version sets a dust relay fee"
        );
    }
}
