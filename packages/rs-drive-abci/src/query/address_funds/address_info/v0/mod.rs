use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_address_info_request::GetAddressInfoRequestV0;
use dapi_grpc::platform::v0::get_address_info_response::{
    get_address_info_response_v0, GetAddressInfoResponseV0,
};
use dapi_grpc::platform::v0::{AddressInfoEntry, BalanceAndNonce};
use dpp::address_funds::PlatformAddress;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_address_info_v0(
        &self,
        GetAddressInfoRequestV0 {
            address: address_bytes,
            prove,
        }: GetAddressInfoRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressInfoResponseV0>, Error> {
        let address: PlatformAddress =
            check_validation_result_with_data!(PlatformAddress::from_bytes(&address_bytes)
                .map_err(|e| {
                    QueryError::InvalidArgument(format!("invalid key_of_type: {}", e))
                }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_balance_and_nonce(
                &address,
                None,
                platform_version,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetAddressInfoResponseV0 {
                result: Some(get_address_info_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let balance_and_nonce = self
                .drive
                .fetch_balance_and_nonce(&address, None, platform_version)?
                .map(|(nonce, balance)| BalanceAndNonce { balance, nonce });

            GetAddressInfoResponseV0 {
                result: Some(get_address_info_response_v0::Result::AddressInfoEntry(
                    AddressInfoEntry {
                        address: address_bytes,
                        balance_and_nonce,
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

    const TEST_ADDRESS: PlatformAddress = PlatformAddress::P2pkh([10; 20]);
    const TEST_NONCE: u32 = 5;
    const TEST_BALANCE: u64 = 1_000_000;

    fn setup_address_balance(
        platform: &Platform<crate::rpc::core::MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) {
        let operations = vec![DriveOperation::AddressFundsOperation(
            AddressFundsOperationType::SetBalanceToAddress {
                address: TEST_ADDRESS,
                nonce: TEST_NONCE,
                balance: TEST_BALANCE,
            },
        )];

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
    fn test_invalid_address_bytes() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressInfoRequestV0 {
            address: vec![0xFF, 0xFF], // invalid address bytes
            prove: false,
        };

        let result = platform
            .query_address_info_v0(request, &state, version)
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
    fn test_address_not_found_returns_none_balance() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let unknown_address = PlatformAddress::P2pkh([99; 20]);
        let address_bytes = unknown_address.to_bytes();

        let request = GetAddressInfoRequestV0 {
            address: address_bytes.clone(),
            prove: false,
        };

        let result = platform
            .query_address_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_address_info_response_v0::Result::AddressInfoEntry(entry)) => {
                assert_eq!(entry.address, address_bytes);
                assert!(
                    entry.balance_and_nonce.is_none(),
                    "expected no balance for unknown address"
                );
            }
            other => panic!("expected AddressInfoEntry result, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_address_info_fetch_with_balance() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        setup_address_balance(&platform, version);

        let address_bytes = TEST_ADDRESS.to_bytes();

        let request = GetAddressInfoRequestV0 {
            address: address_bytes.clone(),
            prove: false,
        };

        let result = platform
            .query_address_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_address_info_response_v0::Result::AddressInfoEntry(entry)) => {
                assert_eq!(entry.address, address_bytes);
                let ban = entry.balance_and_nonce.expect("expected balance and nonce");
                assert_eq!(ban.balance, TEST_BALANCE);
                assert_eq!(ban.nonce, TEST_NONCE);
            }
            other => panic!("expected AddressInfoEntry result, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_address_info_proof_with_balance() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        setup_address_balance(&platform, version);

        let address_bytes = TEST_ADDRESS.to_bytes();

        let request = GetAddressInfoRequestV0 {
            address: address_bytes,
            prove: true,
        };

        let result = platform
            .query_address_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        assert!(matches!(
            response.result,
            Some(get_address_info_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_address_info_proof_absence() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let unknown_address = PlatformAddress::P2pkh([99; 20]);
        let address_bytes = unknown_address.to_bytes();

        let request = GetAddressInfoRequestV0 {
            address: address_bytes,
            prove: true,
        };

        let result = platform
            .query_address_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        assert!(matches!(
            response.result,
            Some(get_address_info_response_v0::Result::Proof(_))
        ));
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_address_info_empty_address_bytes() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressInfoRequestV0 {
            address: vec![],
            prove: false,
        };

        let result = platform
            .query_address_info_v0(request, &state, version)
            .expect("should return validation result");

        assert!(
            !result.errors.is_empty(),
            "expected validation errors for empty address"
        );
    }
}
