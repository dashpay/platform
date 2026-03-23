use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_recent_compacted_address_balance_changes_request::GetRecentCompactedAddressBalanceChangesRequestV0;
use dapi_grpc::platform::v0::get_recent_compacted_address_balance_changes_response::{
    get_recent_compacted_address_balance_changes_response_v0,
    GetRecentCompactedAddressBalanceChangesResponseV0,
};
use dapi_grpc::platform::v0::{
    compacted_address_balance_change, AddToCreditsOperations, BlockHeightCreditEntry,
    CompactedAddressBalanceChange, CompactedAddressBalanceUpdateEntries,
    CompactedBlockAddressBalanceChanges,
};
use dpp::balances::credits::BlockAwareCreditOperation;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_recent_compacted_address_balance_changes_v0(
        &self,
        GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height,
            prove,
        }: GetRecentCompactedAddressBalanceChangesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetRecentCompactedAddressBalanceChangesResponseV0>, Error>
    {
        // Limit the number of compacted entries we return (max 25 to stay within proof size limits)
        // Ensure it matches limit set in rs-drive-proof-verifier/src/proof.rs for RecentCompactedAddressBalanceChanges
        let limit = Some(25u16);

        let response = if prove {
            let proof = self.drive.prove_compacted_address_balance_changes(
                start_block_height,
                limit,
                None,
                platform_version,
            )?;

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetRecentCompactedAddressBalanceChangesResponseV0 {
                result: Some(
                    get_recent_compacted_address_balance_changes_response_v0::Result::Proof(proof),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let compacted_address_balance_changes =
                self.drive.fetch_compacted_address_balance_changes(
                    start_block_height,
                    limit,
                    None,
                    platform_version,
                )?;

            // Convert the fetched data to proto format
            let compacted_block_changes: Vec<CompactedBlockAddressBalanceChanges> =
                compacted_address_balance_changes
                    .into_iter()
                    .map(|(start_block, end_block, changes)| {
                        let address_changes: Vec<CompactedAddressBalanceChange> = changes
                            .into_iter()
                            .map(|(address, operation)| {
                                let op = match operation {
                                    BlockAwareCreditOperation::SetCredits(credits) => {
                                        compacted_address_balance_change::Operation::SetCredits(
                                            credits,
                                        )
                                    }
                                    BlockAwareCreditOperation::AddToCreditsOperations(
                                        block_credits_map,
                                    ) => {
                                        let entries: Vec<BlockHeightCreditEntry> =
                                            block_credits_map
                                                .into_iter()
                                                .map(|(block_height, credits)| {
                                                    BlockHeightCreditEntry {
                                                        block_height,
                                                        credits,
                                                    }
                                                })
                                                .collect();
                                        compacted_address_balance_change::Operation::AddToCreditsOperations(
                                            AddToCreditsOperations { entries },
                                        )
                                    }
                                };
                                CompactedAddressBalanceChange {
                                    address: address.to_bytes(),
                                    operation: Some(op),
                                }
                            })
                            .collect();

                        CompactedBlockAddressBalanceChanges {
                            start_block_height: start_block,
                            end_block_height: end_block,
                            changes: address_changes,
                        }
                    })
                    .collect();

            GetRecentCompactedAddressBalanceChangesResponseV0 {
                result: Some(
                    get_recent_compacted_address_balance_changes_response_v0::Result::CompactedAddressBalanceUpdateEntries(
                        CompactedAddressBalanceUpdateEntries { compacted_block_changes },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::CreditOperation;
    use dpp::dashcore::Network;
    use std::collections::BTreeMap;

    const ADDR1: PlatformAddress = PlatformAddress::P2pkh([10; 20]);
    const ADDR2: PlatformAddress = PlatformAddress::P2sh([11; 20]);

    fn store_balance_changes(
        platform: &Platform<crate::rpc::core::MockCoreRPCLike>,
        block_height: u64,
        changes: BTreeMap<PlatformAddress, CreditOperation>,
        platform_version: &PlatformVersion,
    ) {
        platform
            .drive
            .store_address_balances_for_block(
                &changes,
                block_height,
                1000 * block_height,
                None,
                platform_version,
            )
            .expect("expected to store address balances");
    }

    #[test]
    fn test_no_compacted_changes_returns_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_compacted_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_compacted_address_balance_changes_response_v0::Result::CompactedAddressBalanceUpdateEntries(entries)) => {
                assert!(
                    entries.compacted_block_changes.is_empty(),
                    "expected no compacted block changes"
                );
            }
            other => panic!("expected CompactedAddressBalanceUpdateEntries result, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_prove_no_compacted_changes() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height: 0,
            prove: true,
        };

        let result = platform
            .query_recent_compacted_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        assert!(matches!(
            response.result,
            Some(get_recent_compacted_address_balance_changes_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_fetch_compacted_after_storing_enough_blocks_for_compaction() {
        // Store many small blocks so that compaction gets triggered by the
        // store_address_balances_for_block logic. This tests the full pipeline:
        // store -> compact -> fetch compacted.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Store many individual block changes to trigger compaction
        for i in 1..=200 {
            let mut changes = BTreeMap::new();
            changes.insert(ADDR1, CreditOperation::SetCredits(1000 * i));
            store_balance_changes(&platform, i, changes, version);
        }

        let request = GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_compacted_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_compacted_address_balance_changes_response_v0::Result::CompactedAddressBalanceUpdateEntries(entries)) => {
                // After 200 blocks, compaction should have occurred at least once
                // The exact number depends on compaction thresholds
                assert!(
                    !entries.compacted_block_changes.is_empty(),
                    "expected compacted block changes after many blocks stored"
                );

                // Verify structure of compacted entries
                for compacted in &entries.compacted_block_changes {
                    assert!(
                        compacted.start_block_height <= compacted.end_block_height,
                        "start_block should be <= end_block"
                    );
                    assert!(
                        !compacted.changes.is_empty(),
                        "each compacted entry should have changes"
                    );
                    for change in &compacted.changes {
                        assert!(
                            change.operation.is_some(),
                            "each change should have an operation"
                        );
                    }
                }
            }
            other => panic!("expected CompactedAddressBalanceUpdateEntries result, got {:?}", other),
        }
    }

    #[test]
    fn test_prove_compacted_after_storing_blocks() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Store enough blocks to trigger compaction
        for i in 1..=200 {
            let mut changes = BTreeMap::new();
            changes.insert(ADDR1, CreditOperation::SetCredits(1000 * i));
            store_balance_changes(&platform, i, changes, version);
        }

        let request = GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height: 0,
            prove: true,
        };

        let result = platform
            .query_recent_compacted_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        assert!(matches!(
            response.result,
            Some(get_recent_compacted_address_balance_changes_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_fetch_with_start_height_filter() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Store enough blocks to trigger compaction
        for i in 1..=200 {
            let mut changes = BTreeMap::new();
            changes.insert(ADDR1, CreditOperation::SetCredits(1000 * i));
            store_balance_changes(&platform, i, changes, version);
        }

        // Query from a high start block height
        let request = GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height: 10000,
            prove: false,
        };

        let result = platform
            .query_recent_compacted_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_compacted_address_balance_changes_response_v0::Result::CompactedAddressBalanceUpdateEntries(entries)) => {
                // No compacted entries should exist starting from height 10000
                assert!(
                    entries.compacted_block_changes.is_empty(),
                    "expected no compacted changes for future start height"
                );
            }
            other => panic!("expected CompactedAddressBalanceUpdateEntries result, got {:?}", other),
        }
    }

    #[test]
    fn test_fetch_multiple_addresses_in_compacted() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Store blocks with multiple addresses
        for i in 1..=200 {
            let mut changes = BTreeMap::new();
            changes.insert(ADDR1, CreditOperation::SetCredits(1000 * i));
            changes.insert(ADDR2, CreditOperation::AddToCredits(500 * i));
            store_balance_changes(&platform, i, changes, version);
        }

        let request = GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_compacted_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_compacted_address_balance_changes_response_v0::Result::CompactedAddressBalanceUpdateEntries(entries)) => {
                assert!(
                    !entries.compacted_block_changes.is_empty(),
                    "expected compacted entries"
                );
                // Verify that both addresses appear across the compacted entries
                let all_addresses: std::collections::BTreeSet<Vec<u8>> = entries
                    .compacted_block_changes
                    .iter()
                    .flat_map(|c| c.changes.iter().map(|ch| ch.address.clone()))
                    .collect();
                assert!(
                    all_addresses.contains(&ADDR1.to_bytes()),
                    "missing ADDR1 in compacted changes"
                );
                assert!(
                    all_addresses.contains(&ADDR2.to_bytes()),
                    "missing ADDR2 in compacted changes"
                );
            }
            other => panic!("expected CompactedAddressBalanceUpdateEntries result, got {:?}", other),
        }
    }

    #[test]
    fn test_compacted_add_to_credits_operations_branch() {
        // This test specifically exercises the AddToCreditsOperations branch
        // by only storing AddToCredits operations so compaction produces
        // BlockAwareCreditOperation::AddToCreditsOperations.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Store only AddToCredits operations for many blocks
        for i in 1..=200 {
            let mut changes = BTreeMap::new();
            changes.insert(ADDR1, CreditOperation::AddToCredits(100));
            store_balance_changes(&platform, i, changes, version);
        }

        let request = GetRecentCompactedAddressBalanceChangesRequestV0 {
            start_block_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_compacted_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_compacted_address_balance_changes_response_v0::Result::CompactedAddressBalanceUpdateEntries(entries)) => {
                assert!(
                    !entries.compacted_block_changes.is_empty(),
                    "expected compacted block changes"
                );

                // Look for AddToCreditsOperations entries
                let mut found_add_to_credits = false;
                for compacted in &entries.compacted_block_changes {
                    for change in &compacted.changes {
                        match &change.operation {
                            Some(compacted_address_balance_change::Operation::AddToCreditsOperations(ops)) => {
                                found_add_to_credits = true;
                                assert!(
                                    !ops.entries.is_empty(),
                                    "AddToCreditsOperations should have entries"
                                );
                                // Each entry should have a block height and credits
                                for entry in &ops.entries {
                                    assert!(entry.credits > 0, "credits should be positive");
                                }
                            }
                            Some(compacted_address_balance_change::Operation::SetCredits(_)) => {
                                // SetCredits can also appear if the very last block
                                // before compaction was not yet compacted
                            }
                            None => panic!("expected operation to be present"),
                        }
                    }
                }
                assert!(
                    found_add_to_credits,
                    "expected at least one AddToCreditsOperations entry in compacted results"
                );
            }
            other => panic!("expected CompactedAddressBalanceUpdateEntries result, got {:?}", other),
        }
    }
}
