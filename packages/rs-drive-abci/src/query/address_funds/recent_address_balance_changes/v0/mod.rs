use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_recent_address_balance_changes_request::GetRecentAddressBalanceChangesRequestV0;
use dapi_grpc::platform::v0::get_recent_address_balance_changes_response::{
    get_recent_address_balance_changes_response_v0, GetRecentAddressBalanceChangesResponseV0,
};
use dapi_grpc::platform::v0::{
    address_balance_change, AddressBalanceChange, AddressBalanceUpdateEntries,
    BlockAddressBalanceChanges,
};
use dpp::balances::credits::CreditOperation;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_recent_address_balance_changes_v0(
        &self,
        GetRecentAddressBalanceChangesRequestV0 {
            start_height,
            prove,
        }: GetRecentAddressBalanceChangesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetRecentAddressBalanceChangesResponseV0>, Error> {
        // Limit the number of blocks we return
        let limit = Some(100u16);

        let response = if prove {
            let proof = self.drive.prove_recent_address_balance_changes(
                start_height,
                limit,
                None,
                platform_version,
            )?;

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetRecentAddressBalanceChangesResponseV0 {
                result: Some(get_recent_address_balance_changes_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let address_balance_changes = self.drive.fetch_recent_address_balance_changes(
                start_height,
                limit,
                None,
                platform_version,
            )?;

            // Convert the fetched data to proto format
            let block_changes: Vec<BlockAddressBalanceChanges> = address_balance_changes
                .into_iter()
                .map(|(block_height, changes)| {
                    let address_changes: Vec<AddressBalanceChange> = changes
                        .into_iter()
                        .map(|(address, operation)| {
                            let op = match operation {
                                CreditOperation::SetCredits(credits) => {
                                    address_balance_change::Operation::SetBalance(credits)
                                }
                                CreditOperation::AddToCredits(credits) => {
                                    address_balance_change::Operation::AddToBalance(credits)
                                }
                            };
                            AddressBalanceChange {
                                address: address.to_bytes(),
                                operation: Some(op),
                            }
                        })
                        .collect();

                    BlockAddressBalanceChanges {
                        block_height,
                        changes: address_changes,
                    }
                })
                .collect();

            GetRecentAddressBalanceChangesResponseV0 {
                result: Some(
                    get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(
                        AddressBalanceUpdateEntries { block_changes },
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
    fn test_no_balance_changes_returns_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(
                    entries,
                ),
            ) => {
                assert!(
                    entries.block_changes.is_empty(),
                    "expected no block changes"
                );
            }
            other => panic!(
                "expected AddressBalanceUpdateEntries result, got {:?}",
                other
            ),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_fetch_set_credits_operation() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let mut changes = BTreeMap::new();
        changes.insert(ADDR1, CreditOperation::SetCredits(500_000));

        store_balance_changes(&platform, 100, changes, version);

        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(
                    entries,
                ),
            ) => {
                assert_eq!(entries.block_changes.len(), 1);
                let block_change = &entries.block_changes[0];
                assert_eq!(block_change.block_height, 100);
                assert_eq!(block_change.changes.len(), 1);
                let change = &block_change.changes[0];
                assert_eq!(change.address, ADDR1.to_bytes());
                assert!(matches!(
                    change.operation,
                    Some(address_balance_change::Operation::SetBalance(500_000))
                ));
            }
            other => panic!(
                "expected AddressBalanceUpdateEntries result, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_fetch_add_to_credits_operation() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let mut changes = BTreeMap::new();
        changes.insert(ADDR2, CreditOperation::AddToCredits(250_000));

        store_balance_changes(&platform, 200, changes, version);

        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(
                    entries,
                ),
            ) => {
                assert_eq!(entries.block_changes.len(), 1);
                let change = &entries.block_changes[0].changes[0];
                assert_eq!(change.address, ADDR2.to_bytes());
                assert!(matches!(
                    change.operation,
                    Some(address_balance_change::Operation::AddToBalance(250_000))
                ));
            }
            other => panic!(
                "expected AddressBalanceUpdateEntries result, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_fetch_multiple_blocks() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Store changes for two blocks
        let mut changes1 = BTreeMap::new();
        changes1.insert(ADDR1, CreditOperation::SetCredits(100_000));
        store_balance_changes(&platform, 100, changes1, version);

        let mut changes2 = BTreeMap::new();
        changes2.insert(ADDR2, CreditOperation::AddToCredits(200_000));
        store_balance_changes(&platform, 200, changes2, version);

        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(
                    entries,
                ),
            ) => {
                assert_eq!(entries.block_changes.len(), 2);
                // Block changes should be ordered by block height
                assert_eq!(entries.block_changes[0].block_height, 100);
                assert_eq!(entries.block_changes[1].block_height, 200);
            }
            other => panic!(
                "expected AddressBalanceUpdateEntries result, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_fetch_with_start_height_filter() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let mut changes1 = BTreeMap::new();
        changes1.insert(ADDR1, CreditOperation::SetCredits(100_000));
        store_balance_changes(&platform, 100, changes1, version);

        let mut changes2 = BTreeMap::new();
        changes2.insert(ADDR2, CreditOperation::AddToCredits(200_000));
        store_balance_changes(&platform, 200, changes2, version);

        // Query starting from height 200 - should only get the second block
        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 200,
            prove: false,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(
                    entries,
                ),
            ) => {
                assert_eq!(entries.block_changes.len(), 1);
                assert_eq!(entries.block_changes[0].block_height, 200);
            }
            other => panic!(
                "expected AddressBalanceUpdateEntries result, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_prove_no_balance_changes() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 0,
            prove: true,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        assert!(matches!(
            response.result,
            Some(get_recent_address_balance_changes_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_prove_with_balance_changes() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let mut changes = BTreeMap::new();
        changes.insert(ADDR1, CreditOperation::SetCredits(500_000));
        store_balance_changes(&platform, 100, changes, version);

        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 0,
            prove: true,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        assert!(matches!(
            response.result,
            Some(get_recent_address_balance_changes_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_fetch_multiple_addresses_in_one_block() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let mut changes = BTreeMap::new();
        changes.insert(ADDR1, CreditOperation::SetCredits(100_000));
        changes.insert(ADDR2, CreditOperation::AddToCredits(200_000));
        store_balance_changes(&platform, 100, changes, version);

        let request = GetRecentAddressBalanceChangesRequestV0 {
            start_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_address_balance_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(
                    entries,
                ),
            ) => {
                assert_eq!(entries.block_changes.len(), 1);
                assert_eq!(entries.block_changes[0].changes.len(), 2);
            }
            other => panic!(
                "expected AddressBalanceUpdateEntries result, got {:?}",
                other
            ),
        }
    }
}
