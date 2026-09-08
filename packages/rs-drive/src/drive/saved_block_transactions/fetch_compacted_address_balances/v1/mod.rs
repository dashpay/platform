use crate::drive::Drive;
use crate::error::Error;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

use crate::verify::address_funds::verify_compacted_address_balance_changes::CompactedAddressBalanceProof;

impl Drive {
    /// Version 1 implementation for proving compacted address balance changes
    /// — the two-proof [`CompactedAddressBalanceProof`] bincode envelope used
    /// by protocol versions whose `prove_compacted_address_balance_changes`
    /// feature version is 1.
    ///
    /// Uses two independently verifiable proofs:
    /// 1. A descending predecessor proof authenticates which range, if any,
    ///    contains `start_block_height`.
    /// 2. A forward proof starts at that authenticated range (or at the
    ///    request-derived fallback key when no range contains the height).
    ///
    /// This ensures the proof covers all relevant ranges efficiently, and the
    /// matching `verify_compacted_address_balance_changes_v1` decoder can
    /// bind the forward-query start key to independently verified state.
    pub(super) fn prove_compacted_address_balance_changes_v1(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::saved_compacted_block_transactions_address_balances_path_vec();

        // Step 1: Authenticate the predecessor used to select the forward query.
        let mut desc_end_key = Vec::with_capacity(16);
        desc_end_key.extend_from_slice(&start_block_height.to_be_bytes());
        desc_end_key.extend_from_slice(&u64::MAX.to_be_bytes());

        let mut desc_query = Query::new_with_direction(false); // descending
        desc_query.insert_range_to_inclusive(..=desc_end_key);

        let desc_path_query =
            PathQuery::new(path.clone(), SizedQuery::new(desc_query, Some(1), None));

        let (desc_results, _) = self.grove_get_path_query(
            &desc_path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let predecessor_proof = self.grove_get_proved_path_query(
            &desc_path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        // Determine the actual start key for the proved query
        // If we found a containing range, use its exact key
        // Otherwise use (start_block_height, start_block_height) since end_block >= start_block always
        let start_key = if let Some((key, _)) = desc_results.to_key_elements().into_iter().next() {
            if key.len() == 16 {
                let end_block = u64::from_be_bytes(key[8..16].try_into().unwrap());
                // If this range contains start_block_height, use its exact key
                if end_block >= start_block_height {
                    key
                } else {
                    // No containing range, use (start_block_height, start_block_height)
                    let mut key = Vec::with_capacity(16);
                    key.extend_from_slice(&start_block_height.to_be_bytes());
                    key.extend_from_slice(&start_block_height.to_be_bytes());
                    key
                }
            } else {
                let mut key = Vec::with_capacity(16);
                key.extend_from_slice(&start_block_height.to_be_bytes());
                key.extend_from_slice(&start_block_height.to_be_bytes());
                key
            }
        } else {
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key
        };

        // Step 2: Proved ascending query from start_key

        let mut query = Query::new();
        query.insert_range_from(start_key..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let forward_proof = self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        bincode::encode_to_vec(
            CompactedAddressBalanceProof {
                predecessor_proof,
                forward_proof,
            },
            bincode::config::standard()
                .with_big_endian()
                .with_no_limit(),
        )
        .map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode compacted address balance proof: {e}"
            ))))
        })
    }
}
