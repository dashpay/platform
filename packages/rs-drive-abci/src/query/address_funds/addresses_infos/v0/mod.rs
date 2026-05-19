use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_addresses_infos_request::GetAddressesInfosRequestV0;
use dapi_grpc::platform::v0::get_addresses_infos_response::{
    get_addresses_infos_response_v0, GetAddressesInfosResponseV0,
};
use dapi_grpc::platform::v0::{AddressInfoEntries, AddressInfoEntry, BalanceAndNonce};
use dpp::address_funds::PlatformAddress;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use std::collections::BTreeMap;

impl<C> Platform<C> {
    pub(super) fn query_addresses_infos_v0(
        &self,
        GetAddressesInfosRequestV0 { addresses, prove }: GetAddressesInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressesInfosResponseV0>, Error> {
        if addresses.len() > platform_version.drive_abci.query.max_returned_elements as usize {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to get {} addresses infos, maximum is {}",
                    addresses.len(),
                    platform_version.drive_abci.query.max_returned_elements
                )),
            )));
        }
        // Parse all keys of type
        let addresses: Vec<PlatformAddress> = check_validation_result_with_data!(addresses
            .into_iter()
            .map(|bytes| {
                PlatformAddress::from_bytes(&bytes)
                    .map_err(|e| QueryError::InvalidArgument(format!("invalid key_of_type: {}", e)))
            })
            .collect::<Result<Vec<_>, _>>());

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_balances_with_nonces(
                addresses.iter(),
                None,
                platform_version,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetAddressesInfosResponseV0 {
                result: Some(get_addresses_infos_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let addresses_infos: BTreeMap<_, _> =
                self.drive
                    .fetch_balances_with_nonces(addresses.iter(), None, platform_version)?;

            let address_info_entries = addresses_infos
                .into_iter()
                .map(|(address, balance_info)| AddressInfoEntry {
                    address: address.to_bytes(),
                    balance_and_nonce: balance_info
                        .map(|(nonce, balance)| BalanceAndNonce { balance, nonce }),
                })
                .collect();

            GetAddressesInfosResponseV0 {
                result: Some(get_addresses_infos_response_v0::Result::AddressInfoEntries(
                    AddressInfoEntries {
                        address_info_entries,
                    },
                )),
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
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use drive::util::batch::drive_op_batch::{AddressFundsOperationType, DriveOperation};

    const ADDR1: PlatformAddress = PlatformAddress::P2pkh([10; 20]);
    const ADDR2: PlatformAddress = PlatformAddress::P2sh([11; 20]);
    const NONCE1: u32 = 5;
    const NONCE2: u32 = 7;
    const BALANCE1: u64 = 1_000_000;
    const BALANCE2: u64 = 2_000_000;

    fn setup_two_address_balances(
        platform: &Platform<crate::rpc::core::MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) {
        let operations = vec![
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: ADDR1,
                nonce: NONCE1,
                balance: BALANCE1,
            }),
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: ADDR2,
                nonce: NONCE2,
                balance: BALANCE2,
            }),
        ];

        platform
            .drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply operations");
    }

    #[test]
    fn test_invalid_address_in_list() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![vec![0xFF, 0xFF]], // invalid address bytes
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("should return validation result");

        assert!(
            !result.errors.is_empty(),
            "expected validation errors for invalid address"
        );
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("invalid key_of_type")
        ));
    }

    #[test]
    fn test_addresses_exceeding_max_limit_is_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.max_returned_elements as usize;

        let request = GetAddressesInfosRequestV0 {
            addresses: (0..=max)
                .map(|i| PlatformAddress::P2pkh([i as u8; 20]).to_bytes())
                .collect(),
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(
                drive::error::query::QuerySyntaxError::InvalidLimit(_)
            )]
        ));
    }

    #[test]
    fn test_addresses_at_max_limit_is_accepted() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.max_returned_elements as usize;

        let request = GetAddressesInfosRequestV0 {
            addresses: (0..max)
                .map(|i| PlatformAddress::P2pkh([i as u8; 20]).to_bytes())
                .collect(),
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            !result.errors.iter().any(|e| matches!(
                e,
                QueryError::Query(drive::error::query::QuerySyntaxError::InvalidLimit(_))
            )),
            "should not be rejected at exactly the max limit"
        );
    }

    #[test]
    fn test_empty_addresses_list() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![],
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_addresses_infos_response_v0::Result::AddressInfoEntries(entries)) => {
                assert!(
                    entries.address_info_entries.is_empty(),
                    "expected empty entries"
                );
            }
            other => panic!("expected AddressInfoEntries result, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_addresses_infos_fetch_multiple_existing() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        setup_two_address_balances(&platform, version);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![ADDR1.to_bytes(), ADDR2.to_bytes()],
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_addresses_infos_response_v0::Result::AddressInfoEntries(entries)) => {
                assert_eq!(entries.address_info_entries.len(), 2);

                // Verify each address entry and track which were seen
                let mut saw_addr1 = false;
                let mut saw_addr2 = false;
                for entry in &entries.address_info_entries {
                    let ban = entry
                        .balance_and_nonce
                        .as_ref()
                        .expect("expected balance and nonce");
                    if entry.address == ADDR1.to_bytes() {
                        assert!(!saw_addr1, "duplicate ADDR1 entry");
                        saw_addr1 = true;
                        assert_eq!(ban.balance, BALANCE1);
                        assert_eq!(ban.nonce, NONCE1);
                    } else if entry.address == ADDR2.to_bytes() {
                        assert!(!saw_addr2, "duplicate ADDR2 entry");
                        saw_addr2 = true;
                        assert_eq!(ban.balance, BALANCE2);
                        assert_eq!(ban.nonce, NONCE2);
                    } else {
                        panic!("unexpected address in response");
                    }
                }
                assert!(saw_addr1 && saw_addr2, "expected both addresses");
            }
            other => panic!("expected AddressInfoEntries result, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_addresses_infos_fetch_with_unknown_address() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        setup_two_address_balances(&platform, version);

        let unknown = PlatformAddress::P2pkh([99; 20]);
        let request = GetAddressesInfosRequestV0 {
            addresses: vec![ADDR1.to_bytes(), unknown.to_bytes()],
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_addresses_infos_response_v0::Result::AddressInfoEntries(entries)) => {
                assert_eq!(entries.address_info_entries.len(), 2);

                for entry in &entries.address_info_entries {
                    if entry.address == ADDR1.to_bytes() {
                        let ban = entry
                            .balance_and_nonce
                            .as_ref()
                            .expect("expected balance and nonce for addr1");
                        assert_eq!(ban.balance, BALANCE1);
                        assert_eq!(ban.nonce, NONCE1);
                    } else if entry.address == unknown.to_bytes() {
                        assert!(
                            entry.balance_and_nonce.is_none(),
                            "expected no balance for unknown address"
                        );
                    } else {
                        panic!("unexpected address in response");
                    }
                }
            }
            other => panic!("expected AddressInfoEntries result, got {:?}", other),
        }
    }

    #[test]
    fn test_addresses_infos_proof_multiple() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        setup_two_address_balances(&platform, version);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![ADDR1.to_bytes(), ADDR2.to_bytes()],
            prove: true,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        assert!(matches!(
            response.result,
            Some(get_addresses_infos_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_addresses_infos_proof_empty_list() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![],
            prove: true,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("should return validation result");

        // Proving with an empty list of addresses returns a validation error
        // because GroveDB cannot generate a proof for an empty query
        assert!(
            !result.errors.is_empty(),
            "expected validation errors for empty address list proof"
        );
    }

    #[test]
    fn test_addresses_infos_proof_single_existing_address() {
        // Prove path with a single stored address returns a Proof variant.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        setup_two_address_balances(&platform, version);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![ADDR1.to_bytes()],
            prove: true,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let response = result.data.unwrap();
        assert!(matches!(
            response.result,
            Some(get_addresses_infos_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_addresses_infos_proof_with_unknown_address() {
        // Prove path with only unknown addresses still returns a Proof
        // (an absence proof).
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        setup_two_address_balances(&platform, version);

        let unknown = PlatformAddress::P2pkh([42; 20]);
        let request = GetAddressesInfosRequestV0 {
            addresses: vec![unknown.to_bytes()],
            prove: true,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let response = result.data.unwrap();
        assert!(matches!(
            response.result,
            Some(get_addresses_infos_response_v0::Result::Proof(_))
        ));
    }

    #[test]
    fn test_addresses_infos_first_invalid_short_circuits_list() {
        // A list with one malformed address aborts the whole query with
        // InvalidArgument.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![ADDR1.to_bytes(), vec![0xFF, 0xFF]],
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("invalid key_of_type")
        ));
    }

    #[test]
    fn test_addresses_infos_p2sh_address_roundtrip() {
        // Ensure P2SH addresses parse and round-trip correctly alongside P2PKH.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        setup_two_address_balances(&platform, version);

        // ADDR2 is a P2SH address by setup.
        let request = GetAddressesInfosRequestV0 {
            addresses: vec![ADDR2.to_bytes()],
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let response = result.data.unwrap();
        match response.result {
            Some(get_addresses_infos_response_v0::Result::AddressInfoEntries(entries)) => {
                assert_eq!(entries.address_info_entries.len(), 1);
                let entry = &entries.address_info_entries[0];
                assert_eq!(entry.address, ADDR2.to_bytes());
                let ban = entry.balance_and_nonce.as_ref().expect("present");
                assert_eq!(ban.balance, BALANCE2);
                assert_eq!(ban.nonce, NONCE2);
            }
            other => panic!("expected AddressInfoEntries result, got {:?}", other),
        }
    }

    #[test]
    fn test_addresses_infos_empty_bytes_invalid() {
        // Completely empty address bytes should also fail parsing.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressesInfosRequestV0 {
            addresses: vec![vec![]],
            prove: false,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("invalid key_of_type")
        ));
    }

    #[test]
    fn test_addresses_infos_proof_with_mixed_known_unknown() {
        // Prove path with mixed known + unknown addresses returns a proof
        // (covers both slots).
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        setup_two_address_balances(&platform, version);

        let unknown = PlatformAddress::P2sh([55; 20]);
        let request = GetAddressesInfosRequestV0 {
            addresses: vec![ADDR1.to_bytes(), unknown.to_bytes()],
            prove: true,
        };

        let result = platform
            .query_addresses_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(matches!(
            result.data.unwrap().result,
            Some(get_addresses_infos_response_v0::Result::Proof(_))
        ));
    }
}
