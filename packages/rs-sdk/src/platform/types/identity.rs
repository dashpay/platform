//! Identity related types and functions

use crate::delegate_enum;
use crate::sdk::ProofResponseMetadata;
use crate::{
    platform::{proto, Query},
    Error,
};

use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::{
    GetIdentityByNonUniquePublicKeyHashRequestV0, GetIdentityByNonUniquePublicKeyHashRequestV1,
};
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::Version as NonUniqueResponseVersion;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0;
use dapi_grpc::platform::v0::get_identity_nonce_request::GetIdentityNonceRequestV0;
use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
use dapi_grpc::platform::v0::{
    get_identities_balances_request, get_identity_balance_and_revision_request,
    get_identity_balance_request, get_identity_by_public_key_hash_request,
    get_identity_contract_nonce_request, get_identity_nonce_request, get_identity_request,
    GetIdentitiesBalancesRequest, GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceRequest,
    GetIdentityByPublicKeyHashRequest, GetIdentityContractNonceRequest, GetIdentityNonceRequest,
    GetIdentityRequest, ResponseMetadata,
};
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::TransportError;

// Create enum [IdentityRequest] and [IdentityResponse] that will wrap all possible
// request/response types for [Identity] object.
delegate_enum! {
    IdentityRequest,
    IdentityResponse,
    Identity,
    (GetIdentity,proto::GetIdentityRequest,proto::GetIdentityResponse),
    (GetIdentityByPublicKeyHash, proto::GetIdentityByPublicKeyHashRequest, proto::GetIdentityByPublicKeyHashResponse),
    (GetIdentityByNonUniquePublicKeyHash, proto::GetIdentityByNonUniquePublicKeyHashRequest, proto::GetIdentityByNonUniquePublicKeyHashResponse)
}

impl ProofResponseMetadata for IdentityResponse {
    fn response_metadata(&self) -> Result<&ResponseMetadata, drive_proof_verifier::Error> {
        match self {
            Self::GetIdentity(response) => response.response_metadata(),
            Self::GetIdentityByPublicKeyHash(response) => response.response_metadata(),
            // V0 carries two proofs, so this response has no generic proof accessor.
            Self::GetIdentityByNonUniquePublicKeyHash(response) => match &response.version {
                Some(NonUniqueResponseVersion::V0(v0)) => v0.metadata.as_ref(),
                Some(NonUniqueResponseVersion::V1(v1)) => v1.metadata.as_ref(),
                None => {
                    return Err(drive_proof_verifier::Error::ResponseDecodeError {
                        error: "result did not have a version".to_string(),
                    })
                }
            }
            .ok_or_else(|| drive_proof_verifier::Error::ResponseDecodeError {
                error: "result did not have metadata".to_string(),
            }),
            Self::Unknown => Err(drive_proof_verifier::Error::EmptyVersion),
        }
    }
}

impl Query<IdentityRequest> for dpp::prelude::Identifier {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<IdentityRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }
        let id = self.to_vec();
        Ok(IdentityRequest::GetIdentity(GetIdentityRequest {
            version: Some(get_identity_request::Version::V0(GetIdentityRequestV0 {
                id,
                prove,
            })),
        }))
    }
}

/// Public key hash that can be used as a [Query] to find an identity.
///
/// You can use [`Fetch::fetch(PublicKeyHash)`](crate::platform::Fetch::fetch()) to fetch an identity
/// by its public key hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKeyHash(pub [u8; 20]);

impl Query<IdentityRequest> for PublicKeyHash {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<IdentityRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }
        let request: GetIdentityByPublicKeyHashRequest = GetIdentityByPublicKeyHashRequest {
            version: Some(get_identity_by_public_key_hash_request::Version::V0(
                GetIdentityByPublicKeyHashRequestV0 {
                    prove,
                    public_key_hash: self.0.to_vec(),
                },
            )),
        };

        Ok(request.into())
    }
}

/// Non-unique public key hash that can be used as a [Query] to find an identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonUniquePublicKeyHashQuery {
    pub key_hash: [u8; 20],
    pub after: Option<[u8; 32]>,
}

impl Query<IdentityRequest> for NonUniquePublicKeyHashQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<IdentityRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }

        // V1 (one proof) is refused below protocol 14 and V0 (two proofs) is served at
        // every version. A refusal bans the node for every method, so V1 goes out only
        // when both the stored version and a version the network has proven by a
        // verified, signed answer select it. Seeded, pinned or persisted versions alone
        // never do, and nothing verified yet means V0.
        let stored = settings.protocol_version;
        let verified = settings.verified_protocol_version;
        let version = if verified == 0 {
            0
        } else {
            let basis = if verified >= stored.protocol_version {
                stored
            } else {
                PlatformVersion::get(verified)?
            };
            basis
                .drive_abci
                .query
                .identity_based_queries
                .identity_by_non_unique_public_key_hash
                .default_current_version
        };
        tracing::debug!(
            target: "dash_sdk::query_encoder",
            version,
            stored = stored.protocol_version,
            verified,
            "encoding GetIdentityByNonUniquePublicKeyHashRequest"
        );

        let public_key_hash = self.key_hash.to_vec();
        let start_after = self.after.map(|a| a.to_vec());
        let version = match version {
            0 => proto::get_identity_by_non_unique_public_key_hash_request::Version::V0(
                GetIdentityByNonUniquePublicKeyHashRequestV0 {
                    public_key_hash,
                    start_after,
                    prove,
                },
            ),
            1 => proto::get_identity_by_non_unique_public_key_hash_request::Version::V1(
                GetIdentityByNonUniquePublicKeyHashRequestV1 {
                    public_key_hash,
                    start_after,
                    prove,
                },
            ),
            version => {
                return Err(Error::Config(format!(
                    "unknown getIdentityByNonUniquePublicKeyHash request version {version}"
                )))
            }
        };

        Ok(proto::GetIdentityByNonUniquePublicKeyHashRequest {
            version: Some(version),
        }
        .into())
    }
}

impl Query<GetIdentityBalanceRequest> for dpp::prelude::Identifier {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetIdentityBalanceRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }
        let id = self.to_vec();

        let request: GetIdentityBalanceRequest = GetIdentityBalanceRequest {
            version: Some(get_identity_balance_request::Version::V0(
                GetIdentityBalanceRequestV0 { id, prove },
            )),
        };

        Ok(request)
    }
}

impl Query<GetIdentityNonceRequest> for dpp::prelude::Identifier {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetIdentityNonceRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }

        let request: GetIdentityNonceRequest = GetIdentityNonceRequest {
            version: Some(get_identity_nonce_request::Version::V0(
                GetIdentityNonceRequestV0 {
                    identity_id: self.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Query<GetIdentityContractNonceRequest>
    for (dpp::prelude::Identifier, dpp::prelude::Identifier)
{
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetIdentityContractNonceRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }
        let (identity_id, contract_id) = self;

        let request: GetIdentityContractNonceRequest = GetIdentityContractNonceRequest {
            version: Some(get_identity_contract_nonce_request::Version::V0(
                GetIdentityContractNonceRequestV0 {
                    identity_id: identity_id.to_vec(),
                    contract_id: contract_id.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Query<GetIdentityBalanceAndRevisionRequest> for dpp::prelude::Identifier {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetIdentityBalanceAndRevisionRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }
        let id = self.to_vec();
        let request: GetIdentityBalanceAndRevisionRequest = GetIdentityBalanceAndRevisionRequest {
            version: Some(get_identity_balance_and_revision_request::Version::V0(
                GetIdentityBalanceAndRevisionRequestV0 { id, prove },
            )),
        };

        Ok(request)
    }
}

impl Query<GetIdentitiesBalancesRequest> for Vec<dpp::prelude::Identifier> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetIdentitiesBalancesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            return Err(Error::Config(
                "unproved queries are not supported for this query; enable proofs".to_string(),
            ));
        }
        let ids = self.iter().map(|a| a.to_vec()).collect();

        let request: GetIdentitiesBalancesRequest = GetIdentitiesBalancesRequest {
            version: Some(get_identities_balances_request::Version::V0(
                GetIdentitiesBalancesRequestV0 { ids, prove },
            )),
        };

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::QuerySettings;
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::Version;
    use dpp::version::PLATFORM_VERSIONS;
    use rs_dapi_client::RequestSettings;

    /// The request version the real encoder sends with this stored and verified version.
    fn encoded_version(stored: u32, verified: u32) -> u16 {
        let settings = QuerySettings {
            request_settings: &RequestSettings::default(),
            protocol_version: PlatformVersion::get(stored).expect("known stored version"),
            verified_protocol_version: verified,
            prove: true,
        };
        let IdentityRequest::GetIdentityByNonUniquePublicKeyHash(request) =
            NonUniquePublicKeyHashQuery {
                key_hash: [1; 20],
                after: Some([2; 32]),
            }
            .query(&settings)
            .expect("encodes")
        else {
            panic!("non-unique lookup request")
        };
        match request.version.expect("version") {
            Version::V0(v0) => {
                assert_eq!(
                    (v0.public_key_hash, v0.start_after, v0.prove),
                    (vec![1; 20], Some(vec![2; 32]), true)
                );
                0
            }
            Version::V1(v1) => {
                assert_eq!(
                    (v1.public_key_hash, v1.start_after, v1.prove),
                    (vec![1; 20], Some(vec![2; 32]), true)
                );
                1
            }
        }
    }

    fn accepts(protocol: &PlatformVersion, request_version: u16) -> bool {
        protocol
            .drive_abci
            .query
            .identity_based_queries
            .identity_by_non_unique_public_key_hash
            .check_version(request_version)
    }

    /// A refused request bans the node for every method, so the version the SDK sends must
    /// be one every node of the network accepts. The SDK picks it from
    /// `min(stored, verified)`, and `verified` is a version the network itself signed, so
    /// every protocol version at or above that basis must accept it; before anything is
    /// verified the SDK sends V0, which every protocol version must accept. This ties the
    /// server tables to the SDK's choice: narrowing a bound, or making a table default to
    /// a request version that older tables refuse, turns it red.
    #[test]
    fn should_only_send_nonunique_request_versions_the_network_accepts() {
        let latest = PlatformVersion::latest().protocol_version;
        let first = PlatformVersion::first().protocol_version;
        for stored in first..=latest {
            for verified in std::iter::once(0).chain(first..=latest + 1) {
                let sent = encoded_version(stored, verified);
                let basis = if verified == 0 {
                    first
                } else {
                    stored.min(verified)
                };
                for protocol in PLATFORM_VERSIONS
                    .iter()
                    .filter(|p| p.protocol_version >= basis)
                {
                    assert!(
                        accepts(protocol, sent),
                        "stored {stored}, verified {verified}: V{sent} is refused by protocol {}",
                        protocol.protocol_version
                    );
                }
            }
        }
    }

    /// The lookup's response has no generic accessor, so its metadata is read per version;
    /// it selects the verification rules before the proof is checked.
    #[test]
    fn should_read_nonunique_response_metadata_of_either_version() {
        use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
            GetIdentityByNonUniquePublicKeyHashResponseV0,
            GetIdentityByNonUniquePublicKeyHashResponseV1,
        };
        use dapi_grpc::platform::v0::{
            GetIdentityByNonUniquePublicKeyHashResponse, ResponseMetadata,
        };
        let metadata = ResponseMetadata {
            protocol_version: 14,
            height: 7,
            ..Default::default()
        };
        let response = |version: Option<NonUniqueResponseVersion>| {
            IdentityResponse::GetIdentityByNonUniquePublicKeyHash(
                GetIdentityByNonUniquePublicKeyHashResponse { version },
            )
        };
        for with_metadata in [Some(metadata.clone()), None] {
            for version in [
                NonUniqueResponseVersion::V0(GetIdentityByNonUniquePublicKeyHashResponseV0 {
                    result: None,
                    metadata: with_metadata.clone(),
                }),
                NonUniqueResponseVersion::V1(GetIdentityByNonUniquePublicKeyHashResponseV1 {
                    result: None,
                    metadata: with_metadata.clone(),
                }),
            ] {
                let read = response(Some(version)).response_metadata().cloned();
                match &with_metadata {
                    Some(metadata) => assert_eq!(read.unwrap(), *metadata),
                    None => assert!(matches!(
                        read,
                        Err(drive_proof_verifier::Error::ResponseDecodeError { error })
                            if error.contains("did not have metadata")
                    )),
                }
            }
        }
        assert!(matches!(
            response(None).response_metadata(),
            Err(drive_proof_verifier::Error::ResponseDecodeError { error })
                if error.contains("did not have a version")
        ));
    }

    /// V1 goes out only when both the stored version and a version the network proved
    /// select it; a seeded, pinned or learned stored version alone never does.
    #[test]
    fn should_send_nonunique_v1_only_after_verifying_a_protocol_that_serves_it() {
        for (stored, verified, expected) in [
            (13, 0, 0),
            (14, 0, 0),
            (14, 13, 0),
            (14, 14, 1),
            (13, 14, 0),
            (13, 15, 0),
            (14, 15, 1),
        ] {
            assert_eq!(
                encoded_version(stored, verified),
                expected,
                "stored {stored}, verified {verified}"
            );
        }
    }
}
