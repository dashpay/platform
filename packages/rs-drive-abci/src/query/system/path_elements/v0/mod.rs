use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_path_elements_request::GetPathElementsRequestV0;
use dapi_grpc::platform::v0::get_path_elements_response::get_path_elements_response_v0::Elements;
use dapi_grpc::platform::v0::get_path_elements_response::{
    get_path_elements_response_v0, GetPathElementsResponseV0,
};
use dpp::check_validation_result_with_data;

use crate::error::query::QueryError;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::RootTree;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;

/// Returns `true` if the given root tree is publicly queryable.
///
/// Only trees containing externally-visible data are allowed. Internal system
/// trees (Misc, Pools, SpentAssetLockTransactions, WithdrawalTransactions,
/// Versions, GroupActions, PreFundedSpecializedBalances, SavedBlockTransactions,
/// AddressBalances) are blocked to prevent information disclosure.
fn is_publicly_queryable_root_tree(root_tree: &RootTree) -> bool {
    matches!(
        root_tree,
        RootTree::DataContractDocuments
            | RootTree::Identities
            | RootTree::UniquePublicKeyHashesToIdentities
            | RootTree::NonUniquePublicKeyKeyHashesToIdentities
            | RootTree::Tokens
            | RootTree::Balances
            | RootTree::Votes
    )
}

impl<C> Platform<C> {
    pub(super) fn query_path_elements_v0(
        &self,
        GetPathElementsRequestV0 { path, keys, prove }: GetPathElementsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetPathElementsResponseV0>, Error> {
        // Validate that the requested path targets an allowed root tree.
        // This prevents external callers from reading internal system data
        // such as total credits, pool balances, or spent asset lock transactions.
        if let Some(first_segment) = path.first() {
            if first_segment.len() != 1 {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!(
                        "root path segment must be exactly 1 byte, got {}",
                        first_segment.len()
                    )),
                ));
            }
            let root_tree_byte = first_segment[0];
            match RootTree::try_from(root_tree_byte) {
                Ok(root_tree) => {
                    if !is_publicly_queryable_root_tree(&root_tree) {
                        return Ok(QueryValidationResult::new_with_error(
                            QueryError::InvalidArgument(format!(
                                "path query not allowed for root tree '{}'",
                                root_tree
                            )),
                        ));
                    }
                }
                Err(_) => {
                    return Ok(QueryValidationResult::new_with_error(
                        QueryError::InvalidArgument(format!(
                            "unknown root tree byte: {}",
                            root_tree_byte
                        )),
                    ));
                }
            }
        } else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("path must not be empty".to_string()),
            ));
        }

        if keys.len() > platform_version.drive_abci.query.max_returned_elements as usize {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to get {} values, maximum is {}",
                    keys.len(),
                    platform_version.drive_abci.query.max_returned_elements
                )),
            )));
        }
        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_elements(
                path,
                keys,
                None,
                platform_version
            ));

            GetPathElementsResponseV0 {
                result: Some(get_path_elements_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let result = check_validation_result_with_data!(self.drive.fetch_elements(
                path,
                keys,
                None,
                platform_version
            ));

            let serialized = result
                .into_iter()
                .map(|element| {
                    element
                        .serialize(&platform_version.drive.grove_version)
                        .map_err(|e| Error::Drive(drive::error::Error::from(e)))
                })
                .collect::<Result<Vec<Vec<u8>>, Error>>()?;

            GetPathElementsResponseV0 {
                result: Some(get_path_elements_response_v0::Result::Elements(Elements {
                    elements: serialized,
                })),
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
    use dpp::dashcore::Network;
    use drive::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
    use drive::drive::RootTree;

    #[test]
    fn test_query_misc_tree_is_blocked() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        platform
            .drive
            .add_to_system_credits(100, None, platform_version)
            .expect("expected to add system credits");

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Misc as u8]],
            keys: vec![TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec()],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        // The query should be rejected because Misc is an internal-only tree
        let errors = response.errors;
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            QueryError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("not allowed"),
                    "expected 'not allowed' in error message, got: {}",
                    msg
                );
            }
            other => panic!("expected InvalidArgument error, got: {:?}", other),
        }
    }

    #[test]
    fn test_query_pools_tree_is_blocked() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Pools as u8]],
            keys: vec![vec![0]],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        let errors = response.errors;
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            QueryError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("not allowed"),
                    "expected 'not allowed' in error message, got: {}",
                    msg
                );
            }
            other => panic!("expected InvalidArgument error, got: {:?}", other),
        }
    }

    #[test]
    fn test_query_identities_tree_is_allowed() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Identities as u8]],
            keys: vec![vec![0; 32]],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        // The query should be accepted (no validation errors).
        // It may return no data if the identity doesn't exist, but the path
        // validation itself should pass.
        assert!(
            response.errors.is_empty(),
            "expected no validation errors for Identities tree, got: {:?}",
            response.errors
        );
    }

    #[test]
    fn test_query_data_contract_documents_tree_is_allowed() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::DataContractDocuments as u8]],
            keys: vec![vec![0; 32]],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            response.errors.is_empty(),
            "expected no validation errors for DataContractDocuments tree, got: {:?}",
            response.errors
        );
    }

    #[test]
    fn test_query_empty_path_is_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPathElementsRequestV0 {
            path: vec![],
            keys: vec![vec![0]],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        let errors = response.errors;
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            QueryError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("empty"),
                    "expected 'empty' in error message, got: {}",
                    msg
                );
            }
            other => panic!("expected InvalidArgument error, got: {:?}", other),
        }
    }

    #[test]
    fn test_query_unknown_root_tree_byte_is_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPathElementsRequestV0 {
            path: vec![vec![255]],
            keys: vec![vec![0]],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        let errors = response.errors;
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            QueryError::InvalidArgument(msg) => {
                assert!(
                    msg.contains("unknown root tree byte"),
                    "expected 'unknown root tree byte' in error message, got: {}",
                    msg
                );
            }
            other => panic!("expected InvalidArgument error, got: {:?}", other),
        }
    }
}
