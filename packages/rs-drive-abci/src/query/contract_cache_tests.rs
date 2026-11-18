//! A contract stored as its schemas is parsed each time it is read from state, every pattern
//! it holds compiled. So a query that reads a contract reads it through the contract cache and
//! caches it on a miss, also when it refuses the request once it has read the contract.

use crate::query::tests::{setup_platform, store_data_contract};
use dapi_grpc::platform::v0::get_contract_document_removals_request::get_contract_document_removals_request_v0::Selection;
use dapi_grpc::platform::v0::get_contract_document_removals_request::{
    GetContractDocumentRemovalsRequestV0, Page,
};
use dapi_grpc::platform::v0::get_contract_fee_pots_request::GetContractFeePotsRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_status_request::GetContractModerationStatusRequestV0;
use dapi_grpc::platform::v0::get_private_document_entries_request::GetPrivateDocumentEntriesRequestV0;
use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_request::{
    ContractTokenInfo, GetTokenPerpetualDistributionLastClaimRequestV0,
};
use dapi_grpc::platform::v0::{
    get_contract_document_removals_request, get_contract_fee_pots_request,
    get_contract_moderation_status_request, get_private_document_entries_request,
    get_token_perpetual_distribution_last_claim_request, ContractModerationList,
    GetContractDocumentRemovalsRequest, GetContractFeePotsRequest,
    GetContractModerationStatusRequest, GetPrivateDocumentEntriesRequest,
    GetTokenPerpetualDistributionLastClaimRequest,
};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::tests::fixtures::get_data_contract_fixture;
use std::sync::Arc;

#[test]
fn should_cache_the_contract_a_query_reads_from_a_cold_cache() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let contract =
        get_data_contract_fixture(None, 0, version.protocol_version).data_contract_owned();
    store_data_contract(&platform, &contract, version);
    let id = contract.id().to_buffer();
    let cache = &platform.drive.cache.data_contracts;

    let queries: Vec<(&str, Box<dyn Fn() + '_>)> = vec![
        (
            // Refused once read, as a Drive error: the type is not private
            "getPrivateDocumentEntries",
            Box::new(|| {
                let request = GetPrivateDocumentEntriesRequest {
                    version: Some(get_private_document_entries_request::Version::V0(
                        GetPrivateDocumentEntriesRequestV0 {
                            data_contract_id: id.to_vec(),
                            document_type_name: "niceDocument".to_string(),
                            start_position: 0,
                            prove: false,
                        },
                    )),
                };
                let _ = platform.query_private_document_entries(request, &state, version);
            }),
        ),
        (
            "getContractDocumentRemovals",
            Box::new(|| {
                let request = GetContractDocumentRemovalsRequest {
                    version: Some(get_contract_document_removals_request::Version::V0(
                        GetContractDocumentRemovalsRequestV0 {
                            contract_id: id.to_vec(),
                            document_type_name: "niceDocument".to_string(),
                            selection: Some(Selection::Page(Page {
                                start_after: None,
                                limit: None,
                            })),
                            prove: false,
                        },
                    )),
                };
                platform
                    .query_contract_document_removals(request, &state, version)
                    .expect("expected the query to run");
            }),
        ),
        (
            "getContractFeePots",
            Box::new(|| {
                let request = GetContractFeePotsRequest {
                    version: Some(get_contract_fee_pots_request::Version::V0(
                        GetContractFeePotsRequestV0 {
                            contract_id: id.to_vec(),
                            prove: false,
                        },
                    )),
                };
                platform
                    .query_contract_fee_pots(request, &state, version)
                    .expect("expected the query to run");
            }),
        ),
        (
            // Refused once read: the contract has no token at position 0
            "getTokenPerpetualDistributionLastClaim",
            Box::new(|| {
                let request = GetTokenPerpetualDistributionLastClaimRequest {
                    version: Some(
                        get_token_perpetual_distribution_last_claim_request::Version::V0(
                            GetTokenPerpetualDistributionLastClaimRequestV0 {
                                token_id: vec![1; 32],
                                contract_info: Some(ContractTokenInfo {
                                    contract_id: id.to_vec(),
                                    token_contract_position: 0,
                                }),
                                identity_id: vec![2; 32],
                                prove: false,
                            },
                        ),
                    ),
                };
                platform
                    .query_token_perpetual_distribution_last_claim(request, &state, version)
                    .expect("expected the query to run");
            }),
        ),
        (
            // Refused once read: the contract keeps no banlist
            "getContractModerationStatus",
            Box::new(|| {
                let request = GetContractModerationStatusRequest {
                    version: Some(get_contract_moderation_status_request::Version::V0(
                        GetContractModerationStatusRequestV0 {
                            contract_id: id.to_vec(),
                            identity_id: vec![2; 32],
                            lists: vec![ContractModerationList::Banlist as i32],
                            prove: false,
                        },
                    )),
                };
                platform
                    .query_contract_moderation_status(request, &state, version)
                    .expect("expected the query to run");
            }),
        ),
    ];

    for (name, query) in queries {
        cache.clear();
        query();
        let read_from_state = cache
            .get(id, false)
            .unwrap_or_else(|| panic!("{name}: the query caches the contract it read"));
        query();
        let read_again = cache
            .get(id, false)
            .unwrap_or_else(|| panic!("{name}: the contract stays cached"));
        assert!(
            Arc::ptr_eq(&read_from_state, &read_again),
            "{name}: the second query is answered with the cached contract"
        );
    }
}
