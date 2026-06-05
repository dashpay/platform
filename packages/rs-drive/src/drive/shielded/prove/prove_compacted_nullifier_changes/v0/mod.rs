use crate::drive::shielded::nullifiers::queries::shielded_compacted_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

/// Builds the 16-byte big-endian compacted key `(start_block, end_block)`.
fn compacted_key(start_block: u64, end_block: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(16);
    key.extend_from_slice(&start_block.to_be_bytes());
    key.extend_from_slice(&end_block.to_be_bytes());
    key
}

impl Drive {
    /// Version 0 implementation for proving compacted nullifier changes.
    ///
    /// The proof must let the verifier authenticate **two** things against the
    /// same root hash (see `verify_compacted_nullifier_changes_v0`):
    ///
    /// 1. A **boundary query** — the single greatest compacted key
    ///    `<= (start_block_height, u64::MAX)`. This is what protects against the
    ///    compacted-range absence-proof attack: a range like `(100, 200)` that
    ///    contains the requested height sorts *before* `(150, 150)`, so the
    ///    verifier cannot trust a forward-only proof to surface it.
    /// 2. A **forward query** — `range_from(start_key..)` where `start_key` is
    ///    the containing range (if any) or `(start_block_height,
    ///    start_block_height)`.
    ///
    /// We discover `start_key` with a non-proving descending query, then emit a
    /// single merged proof (`prove_query_many`) that covers BOTH the boundary
    /// key and the forward range so the verifier's chained queries are both
    /// satisfiable.
    pub(super) fn prove_compacted_nullifier_changes_v0(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = shielded_compacted_nullifiers_path_vec();

        // Step 1: Non-proving descending query to find the greatest compacted
        // key <= (start_block_height, u64::MAX).
        let desc_end_key = compacted_key(start_block_height, u64::MAX);

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

        // `boundary_key` is the authenticated lower-bound anchor the verifier
        // will re-derive from its boundary query. `forward_start` is the lower
        // bound of the ascending result scan.
        let (boundary_key, forward_start) = match desc_results.to_key_elements().into_iter().next()
        {
            Some((key, _)) => {
                if key.len() != 16 {
                    return Err(Error::Protocol(Box::new(
                        ProtocolError::CorruptedSerialization(
                            "invalid compacted block key length, expected 16 bytes".to_string(),
                        ),
                    )));
                }
                let end_block = u64::from_be_bytes(key[8..16].try_into().map_err(|_| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(
                        "invalid compacted key slice".to_string(),
                    )))
                })?);
                // If this range contains start_block_height, the forward scan
                // must begin exactly at it; otherwise it begins at
                // (start_block_height, start_block_height).
                let forward_start = if end_block >= start_block_height {
                    key.clone()
                } else {
                    compacted_key(start_block_height, start_block_height)
                };
                (Some(key), forward_start)
            }
            None => (None, compacted_key(start_block_height, start_block_height)),
        };

        // Step 2: build the proof.
        //
        // The verifier's chained boundary query is a descending
        // `range_to_inclusive(..=(start, MAX))` limit-1 subset query. For it to
        // authenticate `boundary_key` as the true greatest key, the proof must
        // include `boundary_key` as a full node together with the contiguity
        // information around it. We therefore prove a single query that unions the
        // boundary key (as an explicit point) with the forward range.
        match boundary_key {
            Some(boundary_key) => {
                // Cap the proof at the caller's `limit` (+1 for the authenticated
                // boundary point) so a client requesting `prove=true` from an early
                // height cannot force a proof spanning the entire compacted history
                // — the public handler passes `limit = Some(25)` "to stay within
                // proof size limits", and dropping it here was a remote
                // CPU/memory/bandwidth amplification vector. The verifier re-applies
                // `limit` to its forward subset query, so this cap is
                // soundness-neutral. Routed through the transaction-aware proof path
                // (not `prove_query_many`, which ignores the transaction) so an
                // active transaction's snapshot is used consistently.
                let capped_limit = limit.map(|l| l.saturating_add(1));
                let mut query = Query::new();
                query.insert_key(boundary_key);
                query.insert_range_from(forward_start..);
                let path_query =
                    PathQuery::new(path.clone(), SizedQuery::new(query, capped_limit, None));

                self.grove_get_proved_path_query(
                    &path_query,
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                )
            }
            None => {
                // No compacted key <= (start, MAX) exists at all (empty /
                // absence). The forward query alone is a sound absence proof:
                // the verifier's boundary query will authenticate that nothing
                // exists <= (start, MAX), and the forward query authenticates
                // the (possibly empty) ascending results.
                let mut forward_query = Query::new();
                forward_query.insert_range_from(forward_start..);
                let forward_path_query =
                    PathQuery::new(path.clone(), SizedQuery::new(forward_query, limit, None));

                self.grove_get_proved_path_query(
                    &forward_path_query,
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                )
            }
        }
    }
}
