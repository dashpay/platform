use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
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

            GetAddressInfoResponseV0 {
                result: Some(get_address_info_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
