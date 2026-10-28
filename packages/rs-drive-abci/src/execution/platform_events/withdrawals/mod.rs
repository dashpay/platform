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
