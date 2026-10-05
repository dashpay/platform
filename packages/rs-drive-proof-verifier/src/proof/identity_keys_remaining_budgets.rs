use crate::error::MapGroveDbError;
use crate::types::identity_keys_remaining_budgets::IdentityKeysRemainingBudgets;
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_identity_keys_remaining_budgets_request, GetIdentityKeysRemainingBudgetsRequest,
    GetIdentityKeysRemainingBudgetsResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use std::collections::BTreeSet;

/// The identity and key ids a request asks about. The proof is verified against a query rebuilt
/// from them, which answers each key id once, so a request the node would refuse (no key id, or
/// a repeated one) is refused here too, before any verification.
pub(crate) fn parse_request(
    request: GetIdentityKeysRemainingBudgetsRequest,
) -> Result<([u8; 32], Vec<KeyID>), Error> {
    let get_identity_keys_remaining_budgets_request::Version::V0(v0) =
        request.version.ok_or(Error::EmptyVersion)?;
    let identity_id = <[u8; 32]>::try_from(v0.identity_id).map_err(|_| Error::RequestError {
        error: "can't convert identity_id to [u8; 32]".to_string(),
    })?;
    if v0.key_ids.is_empty() {
        return Err(Error::RequestError {
            error: "key_ids must name at least one key".to_string(),
        });
    }
    if v0.key_ids.iter().collect::<BTreeSet<_>>().len() != v0.key_ids.len() {
        return Err(Error::RequestError {
            error: "key_ids must not repeat a key id".to_string(),
        });
    }
    Ok((identity_id, v0.key_ids))
}

impl FromProof<GetIdentityKeysRemainingBudgetsRequest> for IdentityKeysRemainingBudgets {
    type Request = GetIdentityKeysRemainingBudgetsRequest;
    type Response = GetIdentityKeysRemainingBudgetsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let (identity_id, key_ids) = parse_request(request)?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_identity_keys_remaining_budgets(
            supported_grovedb_proof_bytes(&proof, platform_version)?,
            identity_id,
            &key_ids,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider, platform_version)?;

        Ok((Some(result), metadata, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_identity_keys_remaining_budgets_request::GetIdentityKeysRemainingBudgetsRequestV0;

    fn request(
        identity_id: Vec<u8>,
        key_ids: Vec<KeyID>,
    ) -> GetIdentityKeysRemainingBudgetsRequest {
        GetIdentityKeysRemainingBudgetsRequest {
            version: Some(get_identity_keys_remaining_budgets_request::Version::V0(
                GetIdentityKeysRemainingBudgetsRequestV0 {
                    identity_id,
                    key_ids,
                    prove: true,
                },
            )),
        }
    }

    #[test]
    fn should_parse_a_well_formed_request() {
        let (identity_id, key_ids) =
            parse_request(request(vec![4; 32], vec![7, 1])).expect("expected to parse");
        assert_eq!(identity_id, [4; 32]);
        assert_eq!(key_ids, vec![7, 1]);
    }

    #[test]
    fn should_refuse_a_request_the_node_would_refuse_before_verifying_anything() {
        assert!(matches!(
            parse_request(GetIdentityKeysRemainingBudgetsRequest { version: None }),
            Err(Error::EmptyVersion)
        ));
        for (bad_request, expected) in [
            (request(vec![4; 31], vec![1]), "identity_id"),
            (request(vec![4; 32], vec![]), "at least one key"),
            (request(vec![4; 32], vec![2, 3, 2]), "must not repeat"),
        ] {
            assert!(
                matches!(
                    parse_request(bad_request),
                    Err(Error::RequestError { error }) if error.contains(expected)
                ),
                "expected a request error about `{expected}`"
            );
        }
    }
}
