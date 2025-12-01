use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
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
use std::collections::BTreeMap;

impl<C> Platform<C> {
    pub(super) fn query_addresses_infos_v0(
        &self,
        GetAddressesInfosRequestV0 { addresses, prove }: GetAddressesInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressesInfosResponseV0>, Error> {
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

            GetAddressesInfosResponseV0 {
                result: Some(get_addresses_infos_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
