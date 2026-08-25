/// Verified average result. Holds the `(count, sum)` pair recovered
/// from a `CountSumTree` / PCPS proof; client divides to obtain the
/// average. Lights up alongside grovedb PR 670's
/// `AggregateCountAndSumOnRange` primitive.
pub mod document_average;
pub mod document_count;
/// Verified having-range (`GROUP BY … HAVING <aggregate> <op> <value>
/// LIMIT n`) result. One entry per matching group, in axis order, read
/// as a value-bounded range of an indexed tree's per-axis secondary
/// (grovedb PR 657); see the file's docs.
pub mod document_having;
/// Verified ranked (`GROUP BY … ORDER BY <aggregate> LIMIT n
/// [OFFSET m]`) result. One entry per returned group, in ranking order,
/// plus the attested rank the page starts at, read from an indexed
/// tree's per-axis secondary (grovedb PR 657); see the file's docs.
pub mod document_ranked;
/// Per-entry verified average result. One `(in_key, key, count, sum)`
/// tuple per matched group; client divides per-entry to obtain
/// per-group averages.
pub mod document_split_average;
pub mod document_split_count;
/// Per-entry verified sum result (sum-side analog of
/// `document_split_count`). One `(in_key, key, sum)` triple per
/// matched group. Lights up alongside grovedb PR 670.
pub mod document_split_sum;
/// Verified sum result (sum-side analog of `document_count`).
/// Single-value aggregate sum recovered from a sum-tree proof.
/// Lights up alongside grovedb PR 670; see the file's docs.
pub mod document_sum;
pub mod groups;
pub mod identity_token_balance;
pub mod token_contract_info;
pub mod token_direct_purchase;
pub mod token_info;
pub mod token_perpetual_distribution_last_claim;
pub mod token_pre_programmed_distributions;
pub mod token_status;
pub mod token_total_supply;

use crate::from_request::TryFromRequest;
use crate::verify::verify_tenderdash_proof;
use crate::{types::*, ContextProvider, DataContractProvider, Error};
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::get_evonodes_proposed_epoch_blocks_by_range_request_v0::Start;
use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
use dapi_grpc::platform::v0::get_path_elements_request::GetPathElementsRequestV0;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::{
    self, GetProtocolVersionUpgradeVoteStatusRequestV0,
};
use dapi_grpc::platform::v0::security_level_map::KeyKindRequestType as GrpcKeyKind;
use dapi_grpc::platform::v0::{
    get_address_info_request, get_addresses_infos_request,
    get_contested_resource_identity_votes_request, get_data_contract_history_request, get_data_contract_request, get_data_contracts_request, get_document_history_request, get_epochs_info_request, get_evonodes_proposed_epoch_blocks_by_ids_request, get_evonodes_proposed_epoch_blocks_by_range_request, get_finalized_epoch_infos_request, get_identities_balances_request, get_identities_contract_keys_request, get_identity_balance_and_revision_request, get_identity_balance_request, get_identity_by_non_unique_public_key_hash_request,
    get_identity_by_public_key_hash_request, get_identity_contract_nonce_request, get_identity_keys_request, get_identity_nonce_request, get_identity_request, get_path_elements_request, get_prefunded_specialized_balance_request, GetContestedResourceVotersForIdentityRequest, GetContestedResourceVotersForIdentityResponse, GetPathElementsRequest, GetPathElementsResponse, GetProtocolVersionUpgradeStateRequest, GetProtocolVersionUpgradeStateResponse, GetProtocolVersionUpgradeVoteStatusRequest, GetProtocolVersionUpgradeVoteStatusResponse, Proof, ResponseMetadata
};
use dapi_grpc::platform::{
    v0::{self as platform, key_request_type, KeyRequestType as GrpcKeyType},
    VersionedGrpcResponse,
};
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::core_subsidy::NetworkCoreSubsidy;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{Network, ProTxHash};
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::identities_contract_keys::IdentitiesContractKeys;
use dpp::identity::Purpose;
use dpp::platform_value::{self};
use dpp::prelude::{AddressNonce, DataContract, Identifier, Identity};
use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::voting::votes::Vote;
use drive::drive::identity::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, PurposeU8, SecurityLevelU8,
};
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::grovedb::Error as GroveError;
use drive::grovedb::GroveTrunkQueryResult;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::proposer_block_count_query::ProposerQueryType;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQuery;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive::query::{DriveDocumentQuery, VotePollsByEndDateDriveQuery};
use indexmap::IndexMap;
use std::array::TryFromSliceError;
use std::collections::BTreeMap;
use std::num::TryFromIntError;
use crate::error::MapGroveDbError;

/// Parse and verify the received proof and retrieve the requested object, if any.
///
/// Use [`FromProof::maybe_from_proof()`] or [`FromProof::from_proof()`] to parse and verify proofs received
/// from the Dash Platform (including verification of grovedb-generated proofs and cryptographic proofs generated
/// by Tenderdash).
///
/// gRPC responses, received from the Dash Platform in response to requests containing `prove: true`, contain
/// GroveDB proof structure (including encapsulated objects) and metadata required to verify cryptographic proof
/// generated by the Tenderdash. This trait provides methods that parse and verify the proof and retrieve the requested
/// object (or information that the object does not exist) in one step.
///
/// This trait is implemented by several objects defined in [Dash Platform Protocol](dpp), like [Identity],
/// [DataContract], [Documents], etc. It is also implemented by several helper objects from [types] module.
pub trait FromProof<Req> {
    /// Request type for which this trait is implemented.
    type Request;
    /// Response type for which this trait is implemented.
    type Response;

    /// Parse and verify the received proof and retrieve the requested object, if any.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using, Mainnet/Testnet/Devnet or Regtest
    /// * `platform_version`: The platform version that should be used.
    /// * `provider`: A callback implementing [ContextProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(object, metadata))` when the requested object was found in the proof.
    /// * `Ok(None)` when the requested object was not found in the proof; this can be interpreted as proof of non-existence.
    ///   For collections, returns Ok(None) if none of the requested objects were found.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn maybe_from_proof<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<Option<Self>, Error>
    where
        Self: Sized + 'a,
    {
        Self::maybe_from_proof_with_metadata(request, response, network, platform_version, provider)
            .map(|maybe_result| maybe_result.0)
    }

    /// Parse and verify the received proof and retrieve the requested object, if any.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using, Mainnet/Testnet/Devnet or Regtest
    /// * `platform_version`: The platform version that should be used.
    /// * `provider`: A callback implementing [ContextProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(Some((object, metadata)))` when the requested object was found in the proof.
    /// * `Ok(None)` when the requested object was not found in the proof; this can be interpreted as proof of non-existence.
    ///   For collections, returns Ok(None) if none of the requested objects were found.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a;

    /// Retrieve the requested object from the proof.
    ///
    /// Runs full verification of the proof and retrieves enclosed objects.
    ///
    /// This method uses [`FromProof::maybe_from_proof()`] internally and throws an error
    /// if the requested object does not exist in the proof.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using, Mainnet/Testnet/Devnet or Regtest
    /// * `platform_version`: The platform version that should be used.
    /// * `provider`: A callback implementing [ContextProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(object)` when the requested object was found in the proof.
    /// * `Err(Error::DocumentMissingInProof)` when the requested object was not found in the proof.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn from_proof<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<Self, Error>
    where
        Self: Sized + 'a,
    {
        Self::maybe_from_proof(request, response, network, platform_version, provider)?
            .ok_or(Error::NotFound)
    }

    /// Retrieve the requested object from the proof with metadata.
    ///
    /// Runs full verification of the proof and retrieves enclosed objects.
    ///
    /// This method uses [`FromProof::maybe_from_proof_with_metadata()`] internally and throws an error
    /// if the requested object does not exist in the proof.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using, Mainnet/Testnet/Devnet or Regtest
    /// * `platform_version`: The platform version that should be used.
    /// * `provider`: A callback implementing [ContextProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(object, metadata))` when the requested object was found in the proof.
    /// * `Err(Error::DocumentMissingInProof)` when the requested object was not found in the proof.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Self, ResponseMetadata), Error>
    where
        Self: Sized + 'a,
    {
        let (main_item, response_metadata, _) = Self::maybe_from_proof_with_metadata(
            request,
            response,
            network,
            platform_version,
            provider,
        )?;
        Ok((main_item.ok_or(Error::NotFound)?, response_metadata))
    }

    /// Retrieve the requested object from the proof with metadata.
    ///
    /// Runs full verification of the proof and retrieves enclosed objects.
    ///
    /// This method uses [`FromProof::maybe_from_proof_with_metadata()`] internally and throws an error
    /// if the requested object does not exist in the proof.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using, Mainnet/Testnet/Devnet or Regtest
    /// * `platform_version`: The platform version that should be used.
    /// * `provider`: A callback implementing [ContextProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(object, metadata, proof))` when the requested object was found in the proof.
    /// * `Err(Error::DocumentMissingInProof)` when the requested object was not found in the proof.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn from_proof_with_metadata_and_proof<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Self, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let (main_item, response_metadata, proof) = Self::maybe_from_proof_with_metadata(
            request,
            response,
            network,
            platform_version,
            provider,
        )?;
        Ok((main_item.ok_or(Error::NotFound)?, response_metadata, proof))
    }
}

impl FromProof<platform::GetIdentityRequest> for Identity {
    type Request = platform::GetIdentityRequest;
    type Response = platform::GetIdentityResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Identity: Sized + 'a,
    {
        let request: platform::GetIdentityRequest = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let id = match request.version.ok_or(Error::EmptyVersion)? {
            get_identity_request::Version::V0(v0) => {
                Identifier::from_bytes(&v0.id).map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                })?
            }
        };

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_full_identity_by_identity_id(
            &proof.grovedb_proof,
            false,
            id.into_buffer(),
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_identity, mtd.clone(), proof.clone()))
    }
}

// TODO: figure out how to deal with mock::automock
impl FromProof<platform::GetIdentityByPublicKeyHashRequest> for Identity {
    type Request = platform::GetIdentityByPublicKeyHashRequest;
    type Response = platform::GetIdentityByPublicKeyHashResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Identity: 'a,
    {
        let request = request.into();
        let response = response.into();
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let public_key_hash = match request.version.ok_or(Error::EmptyVersion)? {
            get_identity_by_public_key_hash_request::Version::V0(v0) => {
                let public_key_hash: [u8; 20] =
                    v0.public_key_hash
                        .try_into()
                        .map_err(|_| Error::DriveError {
                            error: "Invalid public key hash length".to_string(),
                        })?;
                public_key_hash
            }
        };

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_full_identity_by_unique_public_key_hash(
            &proof.grovedb_proof,
            public_key_hash,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_identity, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetIdentityByNonUniquePublicKeyHashRequest> for Identity {
    type Request = platform::GetIdentityByNonUniquePublicKeyHashRequest;
    type Response = platform::GetIdentityByNonUniquePublicKeyHashResponse;
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
        let request = request.into();
        let response = response.into();
        // Parse response to read proof and metadata
        // note that proof in this case is different
        // let proof = response.proof().or(Err(Error::NoProofInResult))?;
        use platform::get_identity_by_non_unique_public_key_hash_response::{
            get_identity_by_non_unique_public_key_hash_response_v0::Result as V0Result, Version::V0,
        };

        let (proved_response, mtd) = match response.version {
            Some(V0(v0)) => {
                let proof = if let V0Result::Proof(p) = v0.result.ok_or(Error::NoProofInResult)? {
                    p
                } else {
                    return Err(Error::NoProofInResult);
                };

                (proof, v0.metadata.ok_or(Error::EmptyResponseMetadata)?)
            }
            _ => return Err(Error::EmptyResponseMetadata),
        };

        // let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (public_key_hash, after_identity) = match request.version.ok_or(Error::EmptyVersion)? {
            get_identity_by_non_unique_public_key_hash_request::Version::V0(v0) => {
                let public_key_hash =
                    v0.public_key_hash
                        .try_into()
                        .map_err(|_| Error::RequestError {
                            error: "Invalid public key hash length".to_string(),
                        })?;

                let after = v0
                    .start_after
                    .map(|a| {
                        a.try_into().map_err(|_| Error::RequestError {
                            error: "Invalid start_after length".to_string(),
                        })
                    })
                    .transpose()?;
                (public_key_hash, after)
            }
        };

        // we need to convert some data to handle non-default proof structure for this response
        let proof = proved_response
            .grovedb_identity_public_key_hash_proof
            .ok_or(Error::NoProofInResult)?;

        let proof_tuple = IdentityAndNonUniquePublicKeyHashDoubleProof {
            identity_proof: proved_response.identity_proof_bytes,
            identity_id_public_key_hash_proof: proof.grovedb_proof.clone(),
        };

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) =
            Drive::verify_full_identity_by_non_unique_public_key_hash(
                &proof_tuple,
                public_key_hash,
                after_identity,
                platform_version,
            )
            .map_err(|e| match e {
                drive::error::Error::GroveDB(e) => {
                    // If InvalidProof error is returned, extract the path query from it
                    let maybe_query = match e.as_ref() {
                        GroveError::InvalidProof(path_query, ..) => Some(path_query.clone()),
                        _ => None,
                    };

                    Error::GroveDBError {
                        proof_bytes: proof.grovedb_proof.clone(),
                        path_query: maybe_query,
                        height: mtd.height,
                        time_ms: mtd.time_ms,
                        error: e.to_string(),
                    }
                }
                _ => e.into(),
            })?;

        verify_tenderdash_proof(&proof, &mtd, &root_hash, provider)?;

        Ok((maybe_identity, mtd.clone(), proof))
    }
}

impl FromProof<platform::GetIdentityKeysRequest> for IdentityPublicKeys {
    type Request = platform::GetIdentityKeysRequest;
    type Response = platform::GetIdentityKeysResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        IdentityPublicKeys: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (request_type, identity_id, limit, offset) =
            match request.version.ok_or(Error::EmptyVersion)? {
                get_identity_keys_request::Version::V0(v0) => {
                    let request_type = v0.request_type;
                    let identity_id = Identifier::from_bytes(&v0.identity_id)
                        .map_err(|e| Error::ProtocolError {
                            error: e.to_string(),
                        })?
                        .into_buffer();
                    let limit = v0.limit.map(try_u32_to_u16).transpose()?;
                    let offset = v0.offset.map(try_u32_to_u16).transpose()?;
                    (request_type, identity_id, limit, offset)
                }
            };

        let request_type = parse_key_request_type(&request_type)?;

        let key_request = IdentityKeysRequest {
            identity_id,
            request_type,
            limit,
            offset,
        };

        tracing::debug!(?identity_id, "checking proof of identity keys");

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_identity_keys_by_identity_id(
            &proof.grovedb_proof,
            key_request,
            false,
            false,
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        let maybe_keys: Option<IdentityPublicKeys> = if let Some(identity) = maybe_identity {
            if identity.loaded_public_keys.is_empty() {
                None
            } else {
                let mut keys = identity
                    .loaded_public_keys
                    .into_iter()
                    .map(|(k, v)| (k, Some(v.clone())))
                    .collect::<IdentityPublicKeys>();

                let mut not_found = identity
                    .not_found_public_keys
                    .into_iter()
                    .map(|k| (k, None))
                    .collect::<IdentityPublicKeys>();

                keys.append(&mut not_found);

                Some(keys)
            }
        } else {
            None
        };

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_keys, mtd.clone(), proof.clone()))
    }
}

fn parse_key_request_type(request: &Option<GrpcKeyType>) -> Result<KeyRequestType, Error> {
    let key_request_type = request
        .to_owned()
        .ok_or(Error::RequestError {
            error: "missing key request type".to_string(),
        })?
        .request
        .ok_or(Error::RequestError {
            error: "empty request field in key request type".to_string(),
        })?;

    let request_type = match key_request_type {
        key_request_type::Request::AllKeys(_) => KeyRequestType::AllKeys,
        key_request_type::Request::SpecificKeys(specific_keys) => {
            KeyRequestType::SpecificKeys(specific_keys.key_ids)
        }
        key_request_type::Request::SearchKey(search_key) => {
            let purpose = search_key
                .purpose_map
                .iter()
                .map(|(k, v)| {
                     let v = v.security_level_map
                            .iter()
                            .map(|(level, &kind)| {
                                let kt = match GrpcKeyKind::try_from(kind) {
                                    Ok(GrpcKeyKind::CurrentKeyOfKindRequest) => {
                                        Ok(KeyKindRequestType::CurrentKeyOfKindRequest)
                                    }
                                    Ok(GrpcKeyKind::AllKeysOfKindRequest) => {
                                        Ok(KeyKindRequestType::AllKeysOfKindRequest)
                                    }
                                    _ => Err(Error::RequestError {
                                        error: format!("missing requested key type: {}", kind),
                                    }),
                                };
                                match kt  {
                                    Err(e) => Err(e),
                                    Ok(d) => Ok((*level as u8, d))
                                }
                            })
                            .collect::<Result<BTreeMap<SecurityLevelU8,KeyKindRequestType>,Error>>();

                            match v {
                                Err(e) =>Err(e),
                                Ok(d) => Ok((*k as u8,d)),
                            }
                })
                .collect::<Result<BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>,Error>>()?;

            KeyRequestType::SearchKey(purpose)
        }
    };

    Ok(request_type)
}

impl FromProof<platform::GetIdentityNonceRequest> for IdentityNonceFetcher {
    type Request = platform::GetIdentityNonceRequest;
    type Response = platform::GetIdentityNonceResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        IdentityNonceFetcher: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let identity_id =
            match request.version.ok_or(Error::EmptyVersion)? {
                get_identity_nonce_request::Version::V0(v0) => Ok::<Identifier, Error>(
                    Identifier::from_bytes(&v0.identity_id).map_err(|e| Error::ProtocolError {
                        error: e.to_string(),
                    })?,
                ),
            }?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_nonce) = Drive::verify_identity_nonce(
            &proof.grovedb_proof,
            identity_id.into_buffer(),
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            maybe_nonce.map(IdentityNonceFetcher),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::GetIdentityContractNonceRequest> for IdentityContractNonceFetcher {
    type Request = platform::GetIdentityContractNonceRequest;
    type Response = platform::GetIdentityContractNonceResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        IdentityContractNonceFetcher: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (identity_id, contract_id) = match request.version.ok_or(Error::EmptyVersion)? {
            get_identity_contract_nonce_request::Version::V0(v0) => {
                Ok::<(Identifier, Identifier), Error>((
                    Identifier::from_bytes(&v0.identity_id).map_err(|e| Error::ProtocolError {
                        error: e.to_string(),
                    })?,
                    Identifier::from_bytes(&v0.contract_id).map_err(|e| Error::ProtocolError {
                        error: e.to_string(),
                    })?,
                ))
            }
        }?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_identity_contract_nonce(
            &proof.grovedb_proof,
            identity_id.into_buffer(),
            contract_id.into_buffer(),
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            maybe_identity.map(IdentityContractNonceFetcher),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::GetIdentityBalanceRequest> for IdentityBalance {
    type Request = platform::GetIdentityBalanceRequest;
    type Response = platform::GetIdentityBalanceResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        IdentityBalance: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let id = match request.version.ok_or(Error::EmptyVersion)? {
            get_identity_balance_request::Version::V0(v0) => Identifier::from_bytes(&v0.id)
                .map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                }),
        }?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_identity_balance_for_identity_id(
            &proof.grovedb_proof,
            id.into_buffer(),
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_identity, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetIdentitiesBalancesRequest> for IdentityBalances {
    type Request = platform::GetIdentitiesBalancesRequest;
    type Response = platform::GetIdentitiesBalancesResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        IdentityBalances: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let identities_ids = match request.version.ok_or(Error::EmptyVersion)? {
            get_identities_balances_request::Version::V0(v0) => v0.ids,
        };

        let identity_ids = identities_ids
            .into_iter()
            .map(|identity_bytes| {
                Identifier::from_bytes(&identity_bytes)
                    .map(|identifier| identifier.into_buffer())
                    .map_err(|e| Error::RequestError {
                        error: format!("identities must be all 32 bytes {}", e),
                    })
            })
            .collect::<Result<Vec<[u8; 32]>, Error>>()?;
        let (root_hash, balances) = Drive::verify_identity_balances_for_identity_ids(
            &proof.grovedb_proof,
            false,
            &identity_ids,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((Some(balances), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetIdentityBalanceAndRevisionRequest> for IdentityBalanceAndRevision {
    type Request = platform::GetIdentityBalanceAndRevisionRequest;
    type Response = platform::GetIdentityBalanceAndRevisionResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        IdentityBalanceAndRevision: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let id = match request.version.ok_or(Error::EmptyVersion)? {
            get_identity_balance_and_revision_request::Version::V0(v0) => {
                Identifier::from_bytes(&v0.id).map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                })
            }
        }?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) =
            Drive::verify_identity_balance_and_revision_for_identity_id(
                &proof.grovedb_proof,
                id.into_buffer(),
                false,
                platform_version,
            )
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_identity, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetAddressInfoRequest> for AddressInfo {
    type Request = platform::GetAddressInfoRequest;
    type Response = platform::GetAddressInfoResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        AddressInfo: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let address = match request.version.ok_or(Error::EmptyVersion)? {
            get_address_info_request::Version::V0(v0) => PlatformAddress::from_bytes(&v0.address)
                .map_err(|e| Error::RequestError {
                error: format!("invalid address: {}", e),
            })?,
        };

        let (root_hash, maybe_info) =
            Drive::verify_address_info(&proof.grovedb_proof, &address, false, platform_version)
                .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let info = maybe_info.map(|(nonce, balance)| AddressInfo {
            address,
            nonce,
            balance,
        });

        Ok((info, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetAddressesInfosRequest> for AddressInfos {
    type Request = platform::GetAddressesInfosRequest;
    type Response = platform::GetAddressesInfosResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        AddressInfos: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let addresses_bytes = match request.version.ok_or(Error::EmptyVersion)? {
            get_addresses_infos_request::Version::V0(v0) => v0.addresses,
        };

        let addresses: Vec<PlatformAddress> = addresses_bytes
            .into_iter()
            .map(|bytes| {
                PlatformAddress::from_bytes(&bytes).map_err(|e| Error::RequestError {
                    error: format!("invalid address: {}", e),
                })
            })
            .collect::<Result<_, _>>()?;

        let (root_hash, entries) = Drive::verify_addresses_infos::<
            _,
            Vec<(PlatformAddress, Option<(AddressNonce, Credits)>)>,
        >(
            &proof.grovedb_proof,
            addresses.iter(),
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let infos = entries
            .into_iter()
            .map(|(address, maybe_info)| {
                let info = maybe_info.map(|(nonce, balance)| AddressInfo {
                    address,
                    nonce,
                    balance,
                });
                (address, info)
            })
            .collect::<AddressInfos>();

        Ok((Some(infos), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetRecentAddressBalanceChangesRequest> for RecentAddressBalanceChanges {
    type Request = platform::GetRecentAddressBalanceChangesRequest;
    type Response = platform::GetRecentAddressBalanceChangesResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        RecentAddressBalanceChanges: 'a,
    {
        use dapi_grpc::platform::v0::get_recent_address_balance_changes_request;

        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (start_height, start_height_exclusive) =
            match request.version.ok_or(Error::EmptyVersion)? {
                get_recent_address_balance_changes_request::Version::V0(v0) => {
                    (v0.start_height, v0.start_height_exclusive)
                }
            };

        let limit = Some(100u16); // Same limit as in query handler

        let (root_hash, verified_changes) = if start_height_exclusive {
            Drive::verify_recent_address_balance_changes_after(
                &proof.grovedb_proof,
                start_height,
                limit,
                false,
                platform_version,
            )
            .map_drive_error(proof, mtd)?
        } else {
            Drive::verify_recent_address_balance_changes(
                &proof.grovedb_proof,
                start_height,
                limit,
                false,
                platform_version,
            )
            .map_drive_error(proof, mtd)?
        };

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let result = RecentAddressBalanceChanges(
            verified_changes
                .into_iter()
                .map(|(block_height, changes)| BlockAddressBalanceChanges {
                    block_height,
                    changes,
                })
                .collect(),
        );

        Ok((Some(result), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetRecentCompactedAddressBalanceChangesRequest>
    for RecentCompactedAddressBalanceChanges
{
    type Request = platform::GetRecentCompactedAddressBalanceChangesRequest;
    type Response = platform::GetRecentCompactedAddressBalanceChangesResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        RecentCompactedAddressBalanceChanges: 'a,
    {
        use dapi_grpc::platform::v0::get_recent_compacted_address_balance_changes_request;

        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let start_block_height = match request.version.ok_or(Error::EmptyVersion)? {
            get_recent_compacted_address_balance_changes_request::Version::V0(v0) => {
                v0.start_block_height
            }
        };

        // Ensure it is the same limit as in query handler; see
        // packages/rs-drive-abci/src/query/address_funds/recent_compacted_address_balance_changes/v0/mod.rs
        let limit = Some(25u16);

        let (root_hash, verified_changes) = Drive::verify_compacted_address_balance_changes(
            &proof.grovedb_proof,
            start_block_height,
            limit,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let result = RecentCompactedAddressBalanceChanges(
            verified_changes
                .into_iter()
                .map(|(start_block_height, end_block_height, changes)| {
                    CompactedBlockAddressBalanceChanges {
                        start_block_height,
                        end_block_height,
                        changes,
                    }
                })
                .collect(),
        );

        Ok((Some(result), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetAddressesTrunkStateRequest> for GroveTrunkQueryResult {
    type Request = platform::GetAddressesTrunkStateRequest;
    type Response = platform::GetAddressesTrunkStateResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        GroveTrunkQueryResult: 'a,
    {
        let response: Self::Response = response.into();

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, trunk_result) =
            Drive::verify_address_funds_trunk_query(&proof.grovedb_proof, platform_version)
                .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((Some(trunk_result), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetAddressesTrunkStateRequest> for PlatformAddressTrunkState {
    type Request = platform::GetAddressesTrunkStateRequest;
    type Response = platform::GetAddressesTrunkStateResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        PlatformAddressTrunkState: 'a,
    {
        let (result, metadata, proof) = <GroveTrunkQueryResult as FromProof<
            platform::GetAddressesTrunkStateRequest,
        >>::maybe_from_proof_with_metadata(
            request, response, network, platform_version, provider
        )?;

        Ok((result.map(PlatformAddressTrunkState), metadata, proof))
    }
}

impl FromProof<platform::GetDataContractRequest> for DataContract {
    type Request = platform::GetDataContractRequest;
    type Response = platform::GetDataContractResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        DataContract: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let id = match request.version.ok_or(Error::EmptyVersion)? {
            get_data_contract_request::Version::V0(v0) => {
                Identifier::from_bytes(&v0.id).map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                })
            }
        }?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_contract) = Drive::verify_contract(
            &proof.grovedb_proof,
            None,
            false,
            false,
            id.into_buffer(),
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_contract, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetDataContractRequest> for (DataContract, Vec<u8>) {
    type Request = platform::GetDataContractRequest;
    type Response = platform::GetDataContractResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        DataContract: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let id = match request.version.ok_or(Error::EmptyVersion)? {
            get_data_contract_request::Version::V0(v0) => {
                Identifier::from_bytes(&v0.id).map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                })
            }
        }?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_contract) = Drive::verify_contract_return_serialization(
            &proof.grovedb_proof,
            None,
            false,
            false,
            id.into_buffer(),
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_contract, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetDataContractsRequest> for DataContracts {
    type Request = platform::GetDataContractsRequest;
    type Response = platform::GetDataContractsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        DataContracts: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let ids = match request.version.ok_or(Error::EmptyVersion)? {
            get_data_contracts_request::Version::V0(v0) => v0.ids,
        };

        let ids = ids
            .iter()
            .map(|id| {
                id.clone().try_into().map_err(|_e| Error::RequestError {
                    error: format!("wrong id size: expected: {}, got: {}", 32, id.len()),
                })
            })
            .collect::<Result<Vec<[u8; 32]>, Error>>()?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, contracts) = Drive::verify_contracts(
            &proof.grovedb_proof,
            false,
            ids.as_slice(),
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;
        let contracts = contracts
            .into_iter()
            .map(|(k, v)| {
                Identifier::from_bytes(&k).map(|id| (id, v)).map_err(|e| {
                    Error::ResultEncodingError {
                        error: e.to_string(),
                    }
                })
            })
            .collect::<Result<DataContracts, Error>>()?;

        let maybe_contracts = if contracts.is_empty() {
            None
        } else {
            Some(contracts)
        };

        Ok((maybe_contracts, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetDataContractHistoryRequest> for DataContractHistory {
    type Request = platform::GetDataContractHistoryRequest;
    type Response = platform::GetDataContractHistoryResponse;

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

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (id, limit, offset, start_at_ms) = match request.version.ok_or(Error::EmptyVersion)? {
            get_data_contract_history_request::Version::V0(v0) => {
                let id = Identifier::from_bytes(&v0.id).map_err(|e| Error::ProtocolError {
                    error: e.to_string(),
                })?;
                let limit = u32_to_u16_opt(v0.limit.unwrap_or_default())?;
                let offset = u32_to_u16_opt(v0.offset.unwrap_or_default())?;
                let start_at_ms = v0.start_at_ms;
                (id, limit, offset, start_at_ms)
            }
        };

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_history) = Drive::verify_contract_history(
            &proof.grovedb_proof,
            id.into_buffer(),
            start_at_ms,
            limit,
            offset,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            maybe_history.map(IndexMap::from_iter),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::GetDocumentHistoryRequest> for DocumentHistory {
    type Request = platform::GetDocumentHistoryRequest;
    type Response = platform::GetDocumentHistoryResponse;

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

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (contract_id, document_type_name, document_id, limit, offset, start_at_ms) =
            match request.version.ok_or(Error::EmptyVersion)? {
                get_document_history_request::Version::V0(v0) => {
                    let contract_id =
                        Identifier::from_bytes(&v0.data_contract_id).map_err(|e| {
                            Error::ProtocolError {
                                error: e.to_string(),
                            }
                        })?;
                    let document_id = Identifier::from_bytes(&v0.document_id).map_err(|e| {
                        Error::ProtocolError {
                            error: e.to_string(),
                        }
                    })?;
                    let limit = u32_to_u16_opt(v0.limit.unwrap_or_default())?;
                    let offset = u32_to_u16_opt(v0.offset.unwrap_or_default())?;
                    (
                        contract_id,
                        v0.document_type_name,
                        document_id,
                        limit,
                        offset,
                        v0.start_at_ms,
                    )
                }
            };

        let data_contract = provider
            .get_data_contract(&contract_id, platform_version)?
            .ok_or(Error::NotFound)?;
        let document_type = data_contract
            .document_type_for_name(&document_type_name)
            .map_err(|e| Error::ProtocolError {
                error: e.to_string(),
            })?;

        let (root_hash, maybe_history) = Drive::verify_document_history(
            &proof.grovedb_proof,
            contract_id.into_buffer(),
            &document_type_name,
            document_type,
            document_id.into_buffer(),
            start_at_ms,
            limit,
            offset,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        // Preserve the distinction between a verified-but-empty history page
        // (e.g. an offset/start_at_ms past the last revision) and an absent
        // result: DocumentHistory carries retrieved values, not proof-of-absence,
        // so a proven empty page is a legitimate `Some(empty)` rather than `None`.
        Ok((
            maybe_history.map(IndexMap::from_iter),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::BroadcastStateTransitionRequest> for StateTransitionProofOutcome {
    type Request = platform::BroadcastStateTransitionRequest;
    type Response = platform::WaitForStateTransitionResultResponse;

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

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let state_transition = StateTransition::deserialize_from_bytes(&request.state_transition)
            .map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let block_info = BlockInfo {
            time_ms: mtd.time_ms,
            height: mtd.height,
            core_height: mtd.core_chain_locked_height,
            // Response metadata is not part of the authenticated state ID.
            // Current proof consumers do not require an epoch, so do not
            // propagate an unsigned selector into the verification context.
            epoch: Default::default(),
        };

        let contracts_provider_fn = provider.as_contract_lookup_fn(platform_version);

        // Transition families whose proof cannot be bound to execution
        // (balance top-ups, credit transfers/withdrawals, address funds
        // movements, shields, no-history token operations) yield a verified
        // snapshot of the affected keys at the proof's block, tagged
        // `AffectedState` on the `StateTransitionProofOutcome`. The tag is
        // preserved here so SDK callers can enforce their required
        // guarantee: strict waits reject snapshots, snapshot waits treat
        // them as height-pinned state, never as execution evidence.
        let (root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            &state_transition,
            &block_info,
            &proof.grovedb_proof,
            &contracts_provider_fn,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((Some(outcome), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetEpochsInfoRequest> for ExtendedEpochInfo {
    type Request = platform::GetEpochsInfoRequest;
    type Response = platform::GetEpochsInfoResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let epochs = ExtendedEpochInfos::maybe_from_proof_with_metadata(
            request,
            response,
            network,
            platform_version,
            provider,
        )?;

        if let Some(e) = epochs.0 {
            if e.len() != 1 {
                return Err(Error::RequestError {
                    error: format!("expected 1 epoch, got {}", e.len()),
                });
            }
            let epoch = e.into_iter().next().and_then(|v| v.1);
            Ok((epoch, epochs.1, epochs.2))
        } else {
            Ok((None, epochs.1, epochs.2))
        }
    }
}

impl FromProof<platform::GetEpochsInfoRequest> for ExtendedEpochInfos {
    type Request = platform::GetEpochsInfoRequest;
    type Response = platform::GetEpochsInfoResponse;

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
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (start_epoch, count, ascending) = match request.version.ok_or(Error::EmptyVersion)? {
            get_epochs_info_request::Version::V0(v0) => (v0.start_epoch, v0.count, v0.ascending),
        };

        let start_epoch: Option<EpochIndex> = if let Some(epoch) = start_epoch {
            Some(try_u32_to_u16(epoch)?)
        } else {
            None
        };
        if start_epoch.is_none() && !ascending {
            return Err(Error::RequestError {
                error: "proved descending epoch queries require an explicit start epoch"
                    .to_string(),
            });
        }
        // This argument is consulted only for descending queries without an
        // explicit start, which are rejected above. Avoid unsigned metadata.
        let current_epoch: EpochIndex = 0;
        let count = try_u32_to_u16(count)?;

        let (root_hash, epoch_info) = Drive::verify_epoch_infos(
            &proof.grovedb_proof,
            current_epoch,
            start_epoch,
            count,
            ascending,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        let epoch_info = epoch_info
            .into_iter()
            .map(|v| {
                #[allow(clippy::infallible_destructuring_match)]
                let info = match &v {
                    ExtendedEpochInfo::V0(i) => i,
                };

                (info.index, Some(v))
            })
            .collect::<ExtendedEpochInfos>();

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((epoch_info.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetFinalizedEpochInfosRequest> for FinalizedEpochInfos {
    type Request = platform::GetFinalizedEpochInfosRequest;
    type Response = platform::GetFinalizedEpochInfosResponse;

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
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (
            start_epoch_index,
            start_epoch_index_included,
            end_epoch_index,
            end_epoch_index_included,
        ) = match request.version.ok_or(Error::EmptyVersion)? {
            get_finalized_epoch_infos_request::Version::V0(v0) => (
                v0.start_epoch_index,
                v0.start_epoch_index_included,
                v0.end_epoch_index,
                v0.end_epoch_index_included,
            ),
        };

        let start_epoch_index: EpochIndex = try_u32_to_u16(start_epoch_index)?;
        let end_epoch_index: EpochIndex = try_u32_to_u16(end_epoch_index)?;

        let (root_hash, epoch_info) = Drive::verify_finalized_epoch_infos(
            &proof.grovedb_proof,
            start_epoch_index,
            start_epoch_index_included,
            end_epoch_index,
            end_epoch_index_included,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        let epoch_info = epoch_info
            .into_iter()
            .map(|(epoch_index, finalized_epoch_info)| (epoch_index, Some(finalized_epoch_info)))
            .collect::<FinalizedEpochInfos>();

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((epoch_info.into_option(), mtd.clone(), proof.clone()))
    }
}

fn try_u32_to_u16(i: u32) -> Result<u16, Error> {
    i.try_into()
        .map_err(|e: TryFromIntError| Error::RequestError {
            error: e.to_string(),
        })
}

impl FromProof<GetProtocolVersionUpgradeStateRequest> for ProtocolVersionUpgrades {
    type Request = GetProtocolVersionUpgradeStateRequest;
    type Response = GetProtocolVersionUpgradeStateResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let response: Self::Response = response.into();
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, objects) =
            Drive::verify_upgrade_state(&proof.grovedb_proof, platform_version)
                .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        // Convert objects to a map of Option values
        let response: Self = objects.into_iter().map(|(k, v)| (k, Some(v))).collect();

        Ok((response.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<GetProtocolVersionUpgradeVoteStatusRequest> for MasternodeProtocolVotes {
    type Request = GetProtocolVersionUpgradeVoteStatusRequest;
    type Response = GetProtocolVersionUpgradeVoteStatusResponse;

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
        let request = request.into();
        let response: Self::Response = response.into();
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let request_v0: GetProtocolVersionUpgradeVoteStatusRequestV0 = match request.version {
            Some(get_protocol_version_upgrade_vote_status_request::Version::V0(v0)) => v0,
            None => return Err(Error::EmptyVersion),
        };

        let start_pro_tx_hash: Option<[u8; 32]> =
            if request_v0.start_pro_tx_hash.is_empty() {
                None
            } else {
                Some(request_v0.start_pro_tx_hash[..].try_into().map_err(
                    |e: TryFromSliceError| Error::RequestError {
                        error: e.to_string(),
                    },
                )?)
            };

        let (root_hash, objects) = Drive::verify_upgrade_vote_status(
            &proof.grovedb_proof,
            start_pro_tx_hash,
            try_u32_to_u16(request_v0.count)?,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        if objects.is_empty() {
            return Ok((None, mtd.clone(), proof.clone()));
        }
        let votes: MasternodeProtocolVotes = objects
            .into_iter()
            .map(|(key, value)| {
                ProTxHash::from_slice(&key)
                    .map(|pro_tx_hash| {
                        (
                            pro_tx_hash,
                            Some(MasternodeProtocolVote {
                                pro_tx_hash,
                                voted_version: value,
                            }),
                        )
                    })
                    .map_err(|e| Error::ResultEncodingError {
                        error: e.to_string(),
                    })
            })
            .collect::<Result<MasternodeProtocolVotes, Error>>()?;

        Ok((votes.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<GetPathElementsRequest> for Elements {
    type Request = GetPathElementsRequest;
    type Response = GetPathElementsResponse;

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
        let request = request.into();
        let response: Self::Response = response.into();
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let request_v0: GetPathElementsRequestV0 = match request.version {
            Some(get_path_elements_request::Version::V0(v0)) => v0,
            None => return Err(Error::EmptyVersion),
        };

        let path = request_v0.path;
        let keys = request_v0.keys;

        let (root_hash, objects) =
            Drive::verify_elements(&proof.grovedb_proof, path, keys, platform_version)?;
        let elements: Elements = Elements::from_iter(objects);

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((elements.into_option(), mtd.clone(), proof.clone()))
    }
}

impl<'dq, Q> FromProof<Q> for Documents
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = platform::GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let request: DriveDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, documents) = request
            .verify_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        let documents = documents
            .into_iter()
            .map(|d| (d.id(), Some(d)))
            .collect::<Documents>();

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((documents.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetIdentitiesContractKeysRequest> for IdentitiesContractKeys {
    type Request = platform::GetIdentitiesContractKeysRequest;
    type Response = platform::GetIdentitiesContractKeysResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (identities_ids, contract_id, document_type_name, purposes) =
            match request.version.ok_or(Error::EmptyVersion)? {
                get_identities_contract_keys_request::Version::V0(v0) => {
                    let GetIdentitiesContractKeysRequestV0 {
                        identities_ids,
                        contract_id,
                        document_type_name,
                        purposes,
                        ..
                    } = v0;
                    let identifiers = identities_ids
                        .into_iter()
                        .map(|identity_id_vec| {
                            let identifier = Identifier::from_vec(identity_id_vec)?;
                            Ok(identifier.to_buffer())
                        })
                        .collect::<Result<Vec<[u8; 32]>, platform_value::Error>>()
                        .map_err(|e| Error::ProtocolError {
                            error: e.to_string(),
                        })?;
                    let contract_id = Identifier::from_vec(contract_id)
                        .map_err(|e| Error::ProtocolError {
                            error: e.to_string(),
                        })?
                        .into_buffer();
                    let purposes = purposes
                        .into_iter()
                        .map(|purpose| {
                            Purpose::try_from(purpose).map_err(|e| Error::ProtocolError {
                                error: e.to_string(),
                            })
                        })
                        .collect::<Result<Vec<Purpose>, Error>>()?;
                    (identifiers, contract_id, document_type_name, purposes)
                }
            };

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, identities_contract_keys) = Drive::verify_identities_contract_keys(
            &proof.grovedb_proof,
            identities_ids.as_slice(),
            &contract_id,
            document_type_name,
            purposes,
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        if identities_contract_keys.is_empty() {
            return Ok((None, mtd.clone(), proof.clone()));
        }

        Ok((Some(identities_contract_keys), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetContestedResourcesRequest> for ContestedResources {
    type Request = platform::GetContestedResourcesRequest;
    type Response = platform::GetContestedResourcesResponse;

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

        // Decode request to get drive query
        let drive_query = VotePollsByDocumentTypeQuery::try_from_request(request)?;
        let resolved_request = drive_query.resolve_with_known_contracts_provider(
            &provider.as_contract_lookup_fn(platform_version),
        )?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, items) = resolved_request
            .verify_contests_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let resources: ContestedResources = items.into_iter().map(ContestedResource).collect();

        Ok((resources.into_option(), mtd.clone(), proof.clone()))
    }
}

// rpc getContestedResourceVoteState(GetContestedResourceVoteStateRequest) returns (GetContestedResourceVoteStateResponse);
impl FromProof<platform::GetContestedResourceVoteStateRequest> for Contenders {
    type Request = platform::GetContestedResourceVoteStateRequest;
    type Response = platform::GetContestedResourceVoteStateResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Decode request to get drive query
        let drive_query = ContestedDocumentVotePollDriveQuery::try_from_request(request)?;

        // Resolve request to get verify_*_proof
        let contracts_provider = provider.as_contract_lookup_fn(platform_version);
        let resolved_request =
            drive_query.resolve_with_known_contracts_provider(&contracts_provider)?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, contested_resource_vote_state) = resolved_request
            .verify_vote_poll_vote_state_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let contenders = contested_resource_vote_state
            .contenders
            .into_iter()
            .map(|v| (v.identity_id(), v))
            .collect();

        let response = Contenders {
            winner: contested_resource_vote_state.winner,
            contenders,
            abstain_vote_tally: contested_resource_vote_state.abstaining_vote_tally,
            lock_vote_tally: contested_resource_vote_state.locked_vote_tally,
        };
        Ok((response.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<GetContestedResourceVotersForIdentityRequest> for Voters {
    type Request = GetContestedResourceVotersForIdentityRequest;
    type Response = GetContestedResourceVotersForIdentityResponse;

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

        // Decode request to get drive query
        let drive_query = ContestedDocumentVotePollVotesDriveQuery::try_from_request(request)?;

        // Parse request to get resolved contract that implements verify_*_proof
        let contracts_provider = provider.as_contract_lookup_fn(platform_version);

        let resolved_request =
            drive_query.resolve_with_known_contracts_provider(&contracts_provider)?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, voters) = resolved_request
            .verify_vote_poll_votes_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        if voters.is_empty() {
            return Ok((None, mtd.clone(), proof.clone()));
        }
        let result: Voters = voters.into_iter().map(Voter::from).collect();

        Ok((result.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetContestedResourceIdentityVotesRequest> for ResourceVotesByIdentity {
    type Request = platform::GetContestedResourceIdentityVotesRequest;
    type Response = platform::GetContestedResourceIdentityVotesResponse;

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

        // Decode request to get drive query
        let drive_query = ContestedResourceVotesGivenByIdentityQuery::try_from_request(request)?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let contract_provider_fn = provider.as_contract_lookup_fn(platform_version);
        let (root_hash, voters) = drive_query
            .verify_identity_votes_given_proof::<Vec<_>>(
                &proof.grovedb_proof,
                &contract_provider_fn,
                platform_version,
            )
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let response: ResourceVotesByIdentity = voters
            .into_iter()
            .map(|(id, vote)| (id, Some(vote)))
            .collect();

        Ok((response.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetVotePollsByEndDateRequest> for VotePollsGroupedByTimestamp {
    type Request = platform::GetVotePollsByEndDateRequest;
    type Response = platform::GetVotePollsByEndDateResponse;

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

        // Decode request to get drive query
        let drive_query = VotePollsByEndDateDriveQuery::try_from_request(request)?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, vote_polls) = drive_query
            .verify_vote_polls_by_end_date_proof::<Vec<(_, _)>>(
                &proof.grovedb_proof,
                platform_version,
            )
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let response = VotePollsGroupedByTimestamp(vote_polls).sorted(drive_query.order_ascending);

        Ok((response.into_option(), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetPrefundedSpecializedBalanceRequest> for PrefundedSpecializedBalance {
    type Request = platform::GetPrefundedSpecializedBalanceRequest;
    type Response = platform::GetPrefundedSpecializedBalanceResponse;

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

        let balance_id = match request.version.ok_or(Error::EmptyVersion)? {
            get_prefunded_specialized_balance_request::Version::V0(v0) => {
                Identifier::from_vec(v0.id).map_err(|e| Error::RequestError {
                    error: e.to_string(),
                })?
            }
        };

        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, balance) = Drive::verify_specialized_balance(
            &proof.grovedb_proof,
            balance_id.into_buffer(),
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((balance.map(|v| v.into()), mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetContestedResourceIdentityVotesRequest> for Vote {
    type Request = platform::GetContestedResourceIdentityVotesRequest;
    type Response = platform::GetContestedResourceIdentityVotesResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request = request.into();
        let id_in_request = match request.version.as_ref().ok_or(Error::EmptyVersion)? {
            get_contested_resource_identity_votes_request::Version::V0(v0) => {
                Identifier::from_bytes(&v0.identity_id).map_err(|e| Error::RequestError {
                    error: e.to_string(),
                })?
            }
        };

        let (maybe_votes, mtd, proof) = ResourceVotesByIdentity::maybe_from_proof_with_metadata(
            request,
            response,
            network,
            platform_version,
            provider,
        )?;

        let (id, vote) = match maybe_votes {
            Some(v) if v.len() > 1 => {
                return Err(Error::ResponseDecodeError {
                    error: format!("expected 1 vote, got {}", v.len()),
                })
            }
            Some(v) if v.is_empty() => return Ok((None, mtd, proof)),
            Some(v) => v
                .into_iter()
                .next()
                .expect("is_empty() must detect empty map"),
            None => return Ok((None, mtd, proof)),
        };

        if id != id_in_request {
            return Err(Error::ResponseDecodeError {
                error: format!(
                    "expected vote for identity {}, got vote for identity {}",
                    id_in_request, id
                ),
            });
        }

        Ok((vote.map(Vote::ResourceVote), mtd, proof))
    }
}

impl FromProof<platform::GetTotalCreditsInPlatformRequest> for TotalCreditsInPlatform {
    type Request = platform::GetTotalCreditsInPlatformRequest;
    type Response = platform::GetTotalCreditsInPlatformResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let response: Self::Response = response.into();
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let core_subsidy_halving_interval = network.core_subsidy_halving_interval();

        let (root_hash, credits) = Drive::verify_total_credits_in_system(
            &proof.grovedb_proof,
            core_subsidy_halving_interval,
            || {
                provider.get_platform_activation_height().map_err(|e| {
                    drive::error::Error::Proof(ProofError::MissingContextRequirement(e.to_string()))
                })
            },
            mtd.core_chain_locked_height,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            Some(TotalCreditsInPlatform(credits)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}
impl FromProof<platform::GetEvonodesProposedEpochBlocksByIdsRequest> for ProposerBlockCounts {
    type Request = platform::GetEvonodesProposedEpochBlocksByIdsRequest;
    type Response = platform::GetEvonodesProposedEpochBlocksResponse;

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
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (ids, epoch) = match request.version.ok_or(Error::EmptyVersion)? {
            get_evonodes_proposed_epoch_blocks_by_ids_request::Version::V0(v0) => {
                (v0.ids, v0.epoch)
            }
        };

        let epoch_index = match epoch {
            Some(index) => try_u32_to_u16(index)?,
            None => {
                return Err(Error::RequestError {
                    error: "proved proposer queries require an explicit epoch".to_string(),
                })
            }
        };

        let (root_hash, proposer_block_counts) = Drive::verify_epoch_proposers(
            &proof.grovedb_proof,
            epoch_index,
            ProposerQueryType::ByIds(ids),
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            Some(ProposerBlockCounts(proposer_block_counts)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::GetEvonodesProposedEpochBlocksByRangeRequest> for ProposerBlockCounts {
    type Request = platform::GetEvonodesProposedEpochBlocksByRangeRequest;
    type Response = platform::GetEvonodesProposedEpochBlocksResponse;

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
        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (epoch, limit, start) = match request.version.ok_or(Error::EmptyVersion)? {
            get_evonodes_proposed_epoch_blocks_by_range_request::Version::V0(v0) => {
                (v0.epoch, v0.limit, v0.start)
            }
        };

        let formatted_start = match start {
            None => None,
            Some(Start::StartAfter(after)) => {
                let id: [u8; 32] = after.try_into().map_err(|_| Error::DriveError {
                    error: "Invalid public key hash length".to_string(),
                })?;
                Some((id, false))
            }
            Some(Start::StartAt(at)) => {
                let id: [u8; 32] = at.try_into().map_err(|_| Error::DriveError {
                    error: "Invalid public key hash length".to_string(),
                })?;
                Some((id, true))
            }
        };

        let epoch_index = match epoch {
            Some(index) => try_u32_to_u16(index)?,
            None => {
                return Err(Error::RequestError {
                    error: "proved proposer queries require an explicit epoch".to_string(),
                })
            }
        };
        let checked_limit = limit.map(try_u32_to_u16).transpose()?;

        let (root_hash, proposer_block_counts) = Drive::verify_epoch_proposers(
            &proof.grovedb_proof,
            epoch_index,
            ProposerQueryType::ByRange(checked_limit, formatted_start),
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            Some(ProposerBlockCounts(proposer_block_counts)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

/// Convert u32, if 0 return None, otherwise return Some(u16).
/// Errors when value is out of range.
fn u32_to_u16_opt(i: u32) -> Result<Option<u16>, Error> {
    let i: Option<u16> = if i != 0 {
        let i: u16 = i
            .try_into()
            .map_err(|e: TryFromIntError| Error::RequestError {
                error: format!("value {} out of range: {}", i, e),
            })?;
        Some(i)
    } else {
        None
    };

    Ok(i)
}

// --- Shielded Pool Query Proof Verification ---

impl FromProof<platform::GetShieldedPoolStateRequest> for ShieldedPoolState {
    type Request = platform::GetShieldedPoolStateRequest;
    type Response = platform::GetShieldedPoolStateResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let response: Self::Response = response.into();
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, maybe_balance) =
            Drive::verify_shielded_pool_state(&proof.grovedb_proof, false, platform_version)
                .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            maybe_balance.map(ShieldedPoolState),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::GetShieldedNotesCountRequest> for ShieldedNotesCount {
    type Request = platform::GetShieldedNotesCountRequest;
    type Response = platform::GetShieldedNotesCountResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let response: Self::Response = response.into();
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        // Mirrors `ShieldedPoolState` above; the only difference is the
        // proved element type — `verify_shielded_notes_count` decodes
        // `total_count` out of the `CommitmentTree` element rather than a
        // `SumItem` balance.
        let (root_hash, maybe_count) =
            Drive::verify_shielded_notes_count(&proof.grovedb_proof, false, platform_version)
                .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            maybe_count.map(ShieldedNotesCount),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::GetShieldedAnchorsRequest> for ShieldedAnchors {
    type Request = platform::GetShieldedAnchorsRequest;
    type Response = platform::GetShieldedAnchorsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let response: Self::Response = response.into();
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, anchors) =
            Drive::verify_shielded_anchors(&proof.grovedb_proof, false, platform_version)
                .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let result = if anchors.is_empty() {
            None
        } else {
            Some(ShieldedAnchors(anchors))
        };

        Ok((result, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetMostRecentShieldedAnchorRequest> for MostRecentShieldedAnchor {
    type Request = platform::GetMostRecentShieldedAnchorRequest;
    type Response = platform::GetMostRecentShieldedAnchorResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let response: Self::Response = response.into();
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, maybe_anchor) = Drive::verify_most_recent_shielded_anchor(
            &proof.grovedb_proof,
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((
            maybe_anchor.map(MostRecentShieldedAnchor),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl FromProof<platform::GetShieldedEncryptedNotesRequest> for ShieldedEncryptedNotes {
    type Request = platform::GetShieldedEncryptedNotesRequest;
    type Response = platform::GetShieldedEncryptedNotesResponse;

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
        use dapi_grpc::platform::v0::get_shielded_encrypted_notes_request;

        let request: Self::Request = request.into();
        let response: Self::Response = response.into();
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (start_index, count) = match request.version.ok_or(Error::EmptyVersion)? {
            get_shielded_encrypted_notes_request::Version::V0(v0) => (v0.start_index, v0.count),
        };

        let max_elements = platform_version
            .drive_abci
            .query
            .shielded_queries
            .max_query_chunks as u32
            * (1u32 << drive::drive::shielded::paths::SHIELDED_NOTES_CHUNK_POWER);

        let (root_hash, notes, total_count) = Drive::verify_shielded_encrypted_notes(
            &proof.grovedb_proof,
            start_index,
            count,
            max_elements,
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        // `total_count` (the on-chain total note count) is extracted from the
        // same proof, so it is available even when this chunk returned no
        // notes — return the result whenever the proof verified, carrying the
        // count for the sync progress-bar denominator. `None` would only be
        // appropriate if the proof itself were absent, which is handled above.
        let result = Some(ShieldedEncryptedNotes {
            notes: notes
                .into_iter()
                .map(|n| ShieldedEncryptedNote {
                    cmx: n.cmx.to_vec(),
                    nullifier: n.nullifier.to_vec(),
                    cv_net: n.cv_net.to_vec(),
                    encrypted_note: n.encrypted_note.to_vec(),
                })
                .collect(),
            total_count,
        });

        Ok((result, mtd.clone(), proof.clone()))
    }
}

impl FromProof<platform::GetShieldedNullifiersRequest> for ShieldedNullifierStatuses {
    type Request = platform::GetShieldedNullifiersRequest;
    type Response = platform::GetShieldedNullifiersResponse;

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
        use dapi_grpc::platform::v0::get_shielded_nullifiers_request;

        let request: Self::Request = request.into();
        let response: Self::Response = response.into();
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let nullifiers = match request.version.ok_or(Error::EmptyVersion)? {
            get_shielded_nullifiers_request::Version::V0(v0) => v0.nullifiers,
        };

        let (root_hash, statuses) = Drive::verify_shielded_nullifiers(
            &proof.grovedb_proof,
            &nullifiers,
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        let result = if statuses.is_empty() {
            None
        } else {
            Some(ShieldedNullifierStatuses(
                statuses
                    .into_iter()
                    .map(|(nullifier, is_spent)| {
                        let nullifier: [u8; 32] =
                            nullifier
                                .try_into()
                                .map_err(|_| Error::ResultEncodingError {
                                    error: "nullifier from Drive proof is not 32 bytes".to_string(),
                                })?;
                        Ok(ShieldedNullifierStatus {
                            nullifier,
                            is_spent,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?,
            ))
        };

        Ok((result, mtd.clone(), proof.clone()))
    }
}

/// Determine number of non-None elements
pub trait Length {
    /// Return number of non-None elements in the data structure
    fn count_some(&self) -> usize;
    /// Return number of all elements in the data structure, including None
    fn count(&self) -> usize;
}

impl<T: Length> Length for Option<T> {
    fn count_some(&self) -> usize {
        match self {
            None => 0,
            Some(i) => i.count_some(),
        }
    }
    fn count(&self) -> usize {
        match self {
            None => 0,
            Some(i) => i.count(),
        }
    }
}

impl<T> Length for Vec<Option<T>> {
    fn count_some(&self) -> usize {
        self.iter().filter(|v| v.is_some()).count()
    }

    fn count(&self) -> usize {
        self.len()
    }
}

impl<K, T> Length for Vec<(K, Option<T>)> {
    fn count_some(&self) -> usize {
        self.iter().filter(|(_, v)| v.is_some()).count()
    }

    fn count(&self) -> usize {
        self.len()
    }
}

impl<K, T> Length for BTreeMap<K, Option<T>> {
    fn count_some(&self) -> usize {
        self.values().filter(|v| v.is_some()).count()
    }

    fn count(&self) -> usize {
        self.len()
    }
}

impl<K, T> Length for IndexMap<K, Option<T>> {
    fn count_some(&self) -> usize {
        self.values().filter(|v| v.is_some()).count()
    }

    fn count(&self) -> usize {
        self.len()
    }
}

/// Implement Length trait for a type
///
/// # Arguments
///
/// * `$object`: The type for which to implement Length trait
/// * `$len`: A closure that returns the length of the object; if omitted, defaults to 1
macro_rules! define_length {
    ($object:ty,$some:expr,$counter:expr) => {
        impl Length for $object {
            fn count_some(&self) -> usize {
                #[allow(clippy::redundant_closure_call)]
                $some(self)
            }

            fn count(&self) -> usize {
                #[allow(clippy::redundant_closure_call)]
                $counter(self)
            }
        }
    };
    ($object:ty,$some:expr) => {
        define_length!($object, $some, $some);
    };
    ($object:ty) => {
        define_length!($object, |_| 1, |_| 1);
    };
}

define_length!(DataContract);
define_length!(DataContractHistory, |d: &DataContractHistory| d.len());
define_length!(DocumentHistory, |d: &DocumentHistory| d.len());
define_length!(Document);
define_length!(Identity);
define_length!(IdentityBalance);
define_length!(IdentityBalanceAndRevision);
define_length!(
    IdentitiesContractKeys,
    |x: &IdentitiesContractKeys| x.values().map(|v| v.count_some()).sum(),
    |x: &IdentitiesContractKeys| x.len()
);
define_length!(ContestedResources, |x: &ContestedResources| x.0.len());
define_length!(Contenders, |x: &Contenders| x.contenders.len());
define_length!(Voters, |x: &Voters| x.0.len());
define_length!(
    VotePollsGroupedByTimestamp,
    |x: &VotePollsGroupedByTimestamp| x.0.iter().map(|v| v.1.len()).sum(),
    |x: &VotePollsGroupedByTimestamp| x.0.len()
);

/// Convert a type into an Option
trait IntoOption
where
    Self: Sized,
{
    /// For zero-length data structures, return None, otherwise return Some(self).
    ///
    /// In case of a zero-length data structure, the function returns None.
    /// Otherwise, it returns Some(self), even it all values are None. This is to ensure that proof of absence
    /// preserves the keys that are not present in the data structure.
    fn into_option(self) -> Option<Self>;
}

impl<L: Length> IntoOption for L {
    fn into_option(self) -> Option<Self>
    where
        Self: Sized,
    {
        if self.count() == 0 {
            None
        } else {
            Some(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_u32_to_u16_succeeds_for_valid_values() {
        assert_eq!(try_u32_to_u16(0).unwrap(), 0u16);
        assert_eq!(try_u32_to_u16(1).unwrap(), 1u16);
        assert_eq!(try_u32_to_u16(42).unwrap(), 42u16);
        assert_eq!(try_u32_to_u16(u16::MAX as u32).unwrap(), u16::MAX);
    }

    #[test]
    fn try_u32_to_u16_errors_on_overflow() {
        // This is the exact attack vector: epoch 65536 would silently truncate
        // to 0 with `as u16`, allowing a malicious node to serve a proof for
        // epoch 0 while claiming the metadata epoch is 65536.
        let result = try_u32_to_u16(65536);
        assert!(
            result.is_err(),
            "epoch 65536 must not silently truncate to 0"
        );

        let result = try_u32_to_u16(u32::MAX);
        assert!(result.is_err(), "epoch u32::MAX must not silently truncate");

        let result = try_u32_to_u16(100_000);
        assert!(result.is_err(), "epoch 100000 must not silently truncate");
    }

    #[test]
    fn u32_to_u16_opt_succeeds_for_valid_values() {
        assert_eq!(u32_to_u16_opt(0).unwrap(), None);
        assert_eq!(u32_to_u16_opt(1).unwrap(), Some(1u16));
        assert_eq!(u32_to_u16_opt(u16::MAX as u32).unwrap(), Some(u16::MAX));
    }

    #[test]
    fn u32_to_u16_opt_errors_on_overflow() {
        let result = u32_to_u16_opt(65536);
        assert!(
            result.is_err(),
            "value 65536 must not silently truncate to 0"
        );

        let result = u32_to_u16_opt(u32::MAX);
        assert!(result.is_err(), "value u32::MAX must not silently truncate");
    }

    // ---------------------------------------------------------------------
    // Length / IntoOption trait tests
    // ---------------------------------------------------------------------

    #[test]
    fn length_vec_option_counts_some_and_total() {
        let v: Vec<Option<u32>> = vec![Some(1), None, Some(2), None, Some(3)];
        assert_eq!(v.count(), 5);
        assert_eq!(v.count_some(), 3);

        let empty: Vec<Option<u32>> = vec![];
        assert_eq!(empty.count(), 0);
        assert_eq!(empty.count_some(), 0);
    }

    #[test]
    fn length_option_of_length_delegates() {
        let inner: Vec<Option<u32>> = vec![Some(1), None];
        let some_inner: Option<Vec<Option<u32>>> = Some(inner);
        assert_eq!(some_inner.count(), 2);
        assert_eq!(some_inner.count_some(), 1);

        let none_inner: Option<Vec<Option<u32>>> = None;
        assert_eq!(none_inner.count(), 0);
        assert_eq!(none_inner.count_some(), 0);
    }

    #[test]
    fn length_vec_of_key_option_pair() {
        let v: Vec<(u8, Option<u32>)> = vec![(1, Some(10)), (2, None), (3, Some(30)), (4, None)];
        assert_eq!(v.count(), 4);
        assert_eq!(v.count_some(), 2);
    }

    #[test]
    fn length_btreemap_of_option() {
        let mut m: BTreeMap<u8, Option<u32>> = BTreeMap::new();
        m.insert(1, Some(10));
        m.insert(2, None);
        m.insert(3, Some(30));
        assert_eq!(m.count(), 3);
        assert_eq!(m.count_some(), 2);
    }

    #[test]
    fn length_indexmap_of_option() {
        let mut m: IndexMap<u8, Option<u32>> = IndexMap::new();
        m.insert(1, Some(10));
        m.insert(2, None);
        m.insert(3, Some(30));
        m.insert(4, None);
        assert_eq!(m.count(), 4);
        assert_eq!(m.count_some(), 2);
    }

    #[test]
    fn into_option_returns_none_for_empty_and_some_for_nonempty() {
        // Empty collection -> None
        let empty: Vec<Option<u32>> = vec![];
        assert!(empty.into_option().is_none());

        // Non-empty, even if all are None -> Some(self)
        let all_none: Vec<Option<u32>> = vec![None, None];
        let wrapped = all_none.into_option();
        assert!(wrapped.is_some());
        assert_eq!(wrapped.unwrap().len(), 2);

        // Non-empty with some values -> Some(self)
        let mixed: Vec<Option<u32>> = vec![Some(1), None];
        assert!(mixed.into_option().is_some());
    }

    #[test]
    fn into_option_for_indexmap() {
        let empty: IndexMap<u8, Option<u32>> = IndexMap::new();
        assert!(empty.into_option().is_none());

        let mut m: IndexMap<u8, Option<u32>> = IndexMap::new();
        m.insert(1, None); // only None value, but count() > 0
        let wrapped = m.into_option();
        assert!(
            wrapped.is_some(),
            "IntoOption must preserve maps that carry absence markers"
        );
    }

    // ---------------------------------------------------------------------
    // parse_key_request_type tests
    // ---------------------------------------------------------------------

    #[test]
    fn parse_key_request_type_missing_outer_request() {
        let err = parse_key_request_type(&None)
            .err()
            .expect("None input must error");
        match err {
            Error::RequestError { error } => {
                assert!(
                    error.contains("missing key request type"),
                    "unexpected error message: {error}"
                );
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn parse_key_request_type_missing_inner_request_field() {
        // Outer Some, inner `request` is None -> second `ok_or` triggers.
        let outer = Some(GrpcKeyType { request: None });
        let err = parse_key_request_type(&outer)
            .err()
            .expect("missing request must error");
        match err {
            Error::RequestError { error } => {
                assert!(
                    error.contains("empty request field"),
                    "unexpected error message: {error}"
                );
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn parse_key_request_type_all_keys_variant() {
        use dapi_grpc::platform::v0::AllKeys;
        let outer = Some(GrpcKeyType {
            request: Some(key_request_type::Request::AllKeys(AllKeys {})),
        });
        let parsed = parse_key_request_type(&outer).unwrap();
        assert!(matches!(parsed, KeyRequestType::AllKeys));
    }

    #[test]
    fn parse_key_request_type_specific_keys_variant() {
        use dapi_grpc::platform::v0::SpecificKeys;
        let outer = Some(GrpcKeyType {
            request: Some(key_request_type::Request::SpecificKeys(SpecificKeys {
                key_ids: vec![1, 2, 3],
            })),
        });
        let parsed = parse_key_request_type(&outer).unwrap();
        match parsed {
            KeyRequestType::SpecificKeys(ids) => assert_eq!(ids, vec![1, 2, 3]),
            _ => panic!("expected SpecificKeys variant"),
        }
    }

    #[test]
    fn parse_key_request_type_search_key_rejects_invalid_kind() {
        use dapi_grpc::platform::v0::{SearchKey, SecurityLevelMap};
        let mut sec_map: std::collections::HashMap<u32, i32> = std::collections::HashMap::new();
        // 99 is not a valid GrpcKeyKind, must produce RequestError
        sec_map.insert(0, 99);

        let mut purpose_map = std::collections::HashMap::new();
        purpose_map.insert(
            0u32,
            SecurityLevelMap {
                security_level_map: sec_map,
            },
        );

        let outer = Some(GrpcKeyType {
            request: Some(key_request_type::Request::SearchKey(SearchKey {
                purpose_map,
            })),
        });

        let err = parse_key_request_type(&outer)
            .err()
            .expect("bad key kind must error");
        match err {
            Error::RequestError { error } => assert!(
                error.contains("missing requested key type"),
                "unexpected error: {error}"
            ),
            other => panic!("expected RequestError for bad key kind, got: {other:?}"),
        }
    }

    #[test]
    fn parse_key_request_type_search_key_accepts_valid_kinds() {
        use dapi_grpc::platform::v0::{SearchKey, SecurityLevelMap};
        let mut sec_map: std::collections::HashMap<u32, i32> = std::collections::HashMap::new();
        sec_map.insert(0, GrpcKeyKind::CurrentKeyOfKindRequest as i32);
        sec_map.insert(1, GrpcKeyKind::AllKeysOfKindRequest as i32);

        let mut purpose_map = std::collections::HashMap::new();
        purpose_map.insert(
            0u32,
            SecurityLevelMap {
                security_level_map: sec_map,
            },
        );

        let outer = Some(GrpcKeyType {
            request: Some(key_request_type::Request::SearchKey(SearchKey {
                purpose_map,
            })),
        });

        let parsed = parse_key_request_type(&outer).unwrap();
        match parsed {
            KeyRequestType::SearchKey(purposes) => {
                let inner = purposes.get(&0u8).expect("purpose 0 parsed");
                assert_eq!(inner.len(), 2);
                assert!(matches!(
                    inner.get(&0u8),
                    Some(KeyKindRequestType::CurrentKeyOfKindRequest)
                ));
                assert!(matches!(
                    inner.get(&1u8),
                    Some(KeyKindRequestType::AllKeysOfKindRequest)
                ));
            }
            _ => panic!("expected SearchKey variant"),
        }
    }

    // ---------------------------------------------------------------------
    // FromProof error-path tests
    //
    // These tests verify that response/request decoding errors fire
    // *before* any cryptographic proof verification is attempted, so we
    // don't need a real quorum or GroveDB proof to exercise them.
    // ---------------------------------------------------------------------

    /// A ContextProvider that must never be called during these tests —
    /// if it is, the test has reached the cryptographic-verification stage
    /// incorrectly, which is itself a meaningful failure.
    struct UnreachableContextProvider;

    impl dash_context_provider::ContextProvider for UnreachableContextProvider {
        fn get_data_contract(
            &self,
            _id: &dpp::prelude::Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<std::sync::Arc<DataContract>>, dash_context_provider::ContextProviderError>
        {
            panic!("context provider should not be called on decode-error test")
        }

        fn get_token_configuration(
            &self,
            _token_id: &dpp::prelude::Identifier,
        ) -> Result<
            Option<dpp::data_contract::TokenConfiguration>,
            dash_context_provider::ContextProviderError,
        > {
            panic!("context provider should not be called on decode-error test")
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], dash_context_provider::ContextProviderError> {
            panic!("context provider should not be called on decode-error test")
        }

        fn get_platform_activation_height(
            &self,
        ) -> Result<dpp::prelude::CoreBlockHeight, dash_context_provider::ContextProviderError>
        {
            panic!("context provider should not be called on decode-error test")
        }
    }

    fn unreachable_provider() -> UnreachableContextProvider {
        UnreachableContextProvider
    }

    fn default_platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    struct NoDataContractProvider;

    impl dash_context_provider::ContextProvider for NoDataContractProvider {
        fn get_data_contract(
            &self,
            _id: &dpp::prelude::Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<std::sync::Arc<DataContract>>, dash_context_provider::ContextProviderError>
        {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _token_id: &dpp::prelude::Identifier,
        ) -> Result<
            Option<dpp::data_contract::TokenConfiguration>,
            dash_context_provider::ContextProviderError,
        > {
            panic!("token configuration should not be requested")
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], dash_context_provider::ContextProviderError> {
            panic!("quorum public key should not be requested")
        }

        fn get_platform_activation_height(
            &self,
        ) -> Result<dpp::prelude::CoreBlockHeight, dash_context_provider::ContextProviderError>
        {
            panic!("platform activation height should not be requested")
        }
    }

    struct StaticDataContractProvider {
        data_contract: std::sync::Arc<DataContract>,
    }

    impl dash_context_provider::ContextProvider for StaticDataContractProvider {
        fn get_data_contract(
            &self,
            id: &dpp::prelude::Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<std::sync::Arc<DataContract>>, dash_context_provider::ContextProviderError>
        {
            if self.data_contract.id() == *id {
                Ok(Some(self.data_contract.clone()))
            } else {
                Ok(None)
            }
        }

        fn get_token_configuration(
            &self,
            _token_id: &dpp::prelude::Identifier,
        ) -> Result<
            Option<dpp::data_contract::TokenConfiguration>,
            dash_context_provider::ContextProviderError,
        > {
            panic!("token configuration should not be requested")
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], dash_context_provider::ContextProviderError> {
            panic!("quorum public key should not be requested")
        }

        fn get_platform_activation_height(
            &self,
        ) -> Result<dpp::prelude::CoreBlockHeight, dash_context_provider::ContextProviderError>
        {
            panic!("platform activation height should not be requested")
        }
    }

    /// Build a fully-populated `GetIdentityResponse` shell so that
    /// `response.proof()` and `response.metadata()` both succeed. The
    /// enclosed proof is empty, so any real verification would fail — but
    /// these tests stop before that point.
    fn identity_response_with_proof_and_metadata() -> platform::GetIdentityResponse {
        use platform::get_identity_response::{
            get_identity_response_v0::Result as V0Result, GetIdentityResponseV0, Version,
        };
        platform::GetIdentityResponse {
            version: Some(Version::V0(GetIdentityResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        }
    }

    fn document_history_response_with_proof_and_metadata() -> platform::GetDocumentHistoryResponse {
        use platform::get_document_history_response::{
            get_document_history_response_v0::Result as V0Result, GetDocumentHistoryResponseV0,
            Version,
        };
        platform::GetDocumentHistoryResponse {
            version: Some(Version::V0(GetDocumentHistoryResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        }
    }

    fn document_history_request(
        data_contract_id: Vec<u8>,
        document_type_name: &str,
        document_id: Vec<u8>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> platform::GetDocumentHistoryRequest {
        use dapi_grpc::platform::v0::get_document_history_request::GetDocumentHistoryRequestV0;
        GetDocumentHistoryRequestV0 {
            data_contract_id,
            document_type_name: document_type_name.to_string(),
            document_id,
            limit,
            offset,
            start_at_ms: 0,
            prove: true,
        }
        .into()
    }

    fn document_history_error(
        request: platform::GetDocumentHistoryRequest,
        response: platform::GetDocumentHistoryResponse,
        provider: &dyn ContextProvider,
    ) -> Error {
        <DocumentHistory as FromProof<
            platform::GetDocumentHistoryRequest,
        >>::maybe_from_proof_with_metadata(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            provider,
        )
        .unwrap_err()
    }

    #[test]
    fn identity_from_proof_no_proof_when_response_empty() {
        // Default response has `version: None` -> response.proof() errors
        // -> mapped to NoProofInResult.
        let request = platform::GetIdentityRequest::default();
        let response = platform::GetIdentityResponse::default();

        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();

        assert!(
            matches!(err, Error::NoProofInResult),
            "expected NoProofInResult, got: {err:?}"
        );
    }

    #[test]
    fn identity_from_proof_empty_metadata_when_metadata_missing() {
        use platform::get_identity_response::{
            get_identity_response_v0::Result as V0Result, GetIdentityResponseV0, Version,
        };
        // Response has a Proof but no metadata -> EmptyResponseMetadata.
        let response = platform::GetIdentityResponse {
            version: Some(Version::V0(GetIdentityResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: None,
            })),
        };
        let request = platform::GetIdentityRequest::default();
        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }

    #[test]
    fn identity_from_proof_empty_version_when_request_has_no_version() {
        // Valid response, but request.version is None -> EmptyVersion.
        let response = identity_response_with_proof_and_metadata();
        let request = platform::GetIdentityRequest { version: None };
        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn identity_from_proof_protocol_error_on_bad_id_length() {
        use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
        // id must be 32 bytes; anything else fails Identifier::from_bytes.
        let request: platform::GetIdentityRequest = GetIdentityRequestV0 {
            id: vec![0u8; 8],
            prove: true,
        }
        .into();
        let response = identity_response_with_proof_and_metadata();
        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::ProtocolError { .. }),
            "expected ProtocolError on bad id length, got: {err:?}"
        );
    }

    /// A minimal `FromProof` impl whose `maybe_from_proof_with_metadata`
    /// returns `Ok((None, ..))`, isolating the `from_proof` wrapper's
    /// `None -> Error::NotFound` mapping from the decode/verify pipeline.
    #[derive(Debug)]
    struct MissingFromProof;

    impl FromProof<()> for MissingFromProof {
        type Request = ();
        type Response = ();

        fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
            _request: I,
            _response: O,
            _network: Network,
            _platform_version: &PlatformVersion,
            _provider: &'a dyn ContextProvider,
        ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
        where
            Self: Sized + 'a,
        {
            Ok((None, ResponseMetadata::default(), Proof::default()))
        }
    }

    #[test]
    fn from_proof_maps_none_to_not_found() {
        // `from_proof` (vs `maybe_from_proof`) is expected to map `Ok(None)`
        // to `Error::NotFound`. Verify that wrapper behavior directly rather
        // than conflating it with decode-error propagation.
        let provider = unreachable_provider();
        let err = <MissingFromProof as FromProof<()>>::from_proof(
            (),
            (),
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::NotFound),
            "expected NotFound when maybe_from_proof returns None, got: {err:?}"
        );
    }

    #[test]
    fn identity_by_public_key_hash_invalid_length_yields_drive_error() {
        use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;

        // public_key_hash must be exactly 20 bytes; 10 bytes fails.
        let request: platform::GetIdentityByPublicKeyHashRequest =
            GetIdentityByPublicKeyHashRequestV0 {
                public_key_hash: vec![0u8; 10],
                prove: true,
            }
            .into();

        // Build a response that succeeds on proof/metadata lookups.
        use platform::get_identity_by_public_key_hash_response::{
            get_identity_by_public_key_hash_response_v0::Result as V0Result,
            GetIdentityByPublicKeyHashResponseV0, Version,
        };
        let response = platform::GetIdentityByPublicKeyHashResponse {
            version: Some(Version::V0(GetIdentityByPublicKeyHashResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };

        let provider = unreachable_provider();
        let err =
            <Identity as FromProof<platform::GetIdentityByPublicKeyHashRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();

        match err {
            Error::DriveError { error } => {
                assert!(
                    error.contains("Invalid public key hash length"),
                    "unexpected error body: {error}"
                );
            }
            other => panic!("expected DriveError, got: {other:?}"),
        }
    }

    #[test]
    fn identity_by_non_unique_public_key_hash_rejects_bad_key_hash_length() {
        use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
        use platform::get_identity_by_non_unique_public_key_hash_response::{
            get_identity_by_non_unique_public_key_hash_response_v0::Result as V0Result,
            GetIdentityByNonUniquePublicKeyHashResponseV0, Version,
        };

        // Build a response with a proved result so we get past the response shape check
        // and hit the request validation.
        let response = platform::GetIdentityByNonUniquePublicKeyHashResponse {
            version: Some(Version::V0(
                GetIdentityByNonUniquePublicKeyHashResponseV0 {
                    result: Some(V0Result::Proof(
                        dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::get_identity_by_non_unique_public_key_hash_response_v0::IdentityProvedResponse {
                            identity_proof_bytes: None,
                            grovedb_identity_public_key_hash_proof: Some(Proof::default()),
                        },
                    )),
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        };

        let request: platform::GetIdentityByNonUniquePublicKeyHashRequest =
            GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: vec![0u8; 3], // must be 20 bytes
                start_after: None,
                prove: true,
            }
            .into();

        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();

        match err {
            Error::RequestError { error } => {
                assert!(
                    error.contains("Invalid public key hash length"),
                    "got: {error}"
                );
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn identity_by_non_unique_public_key_hash_rejects_bad_start_after_length() {
        use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
        use platform::get_identity_by_non_unique_public_key_hash_response::{
            get_identity_by_non_unique_public_key_hash_response_v0::Result as V0Result,
            GetIdentityByNonUniquePublicKeyHashResponseV0, Version,
        };

        let response = platform::GetIdentityByNonUniquePublicKeyHashResponse {
            version: Some(Version::V0(
                GetIdentityByNonUniquePublicKeyHashResponseV0 {
                    result: Some(V0Result::Proof(
                        dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::get_identity_by_non_unique_public_key_hash_response_v0::IdentityProvedResponse {
                            identity_proof_bytes: None,
                            grovedb_identity_public_key_hash_proof: Some(Proof::default()),
                        },
                    )),
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        };

        let request: platform::GetIdentityByNonUniquePublicKeyHashRequest =
            GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: vec![0u8; 20],   // good
                start_after: Some(vec![0u8; 10]), // wrong length; must be 32
                prove: true,
            }
            .into();

        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();

        match err {
            Error::RequestError { error } => {
                assert!(error.contains("Invalid start_after length"), "got: {error}");
            }
            other => panic!("expected RequestError for start_after, got: {other:?}"),
        }
    }

    #[test]
    fn identity_by_non_unique_response_with_no_result_yields_no_proof() {
        use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
        use platform::get_identity_by_non_unique_public_key_hash_response::{
            GetIdentityByNonUniquePublicKeyHashResponseV0, Version,
        };

        // v0 with result=None -> NoProofInResult on the `.ok_or` branch.
        let response = platform::GetIdentityByNonUniquePublicKeyHashResponse {
            version: Some(Version::V0(GetIdentityByNonUniquePublicKeyHashResponseV0 {
                result: None,
                metadata: None,
            })),
        };
        let request: platform::GetIdentityByNonUniquePublicKeyHashRequest =
            GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: vec![0u8; 20],
                start_after: None,
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn identity_by_non_unique_response_with_no_version_yields_empty_metadata() {
        // response.version = None hits the `_ => EmptyResponseMetadata` arm.
        use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
        let response = platform::GetIdentityByNonUniquePublicKeyHashResponse { version: None };
        let request: platform::GetIdentityByNonUniquePublicKeyHashRequest =
            GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash: vec![0u8; 20],
                start_after: None,
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <Identity as FromProof<platform::GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }

    #[test]
    fn identities_balances_rejects_non_32_byte_id() {
        use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
        use platform::get_identities_balances_response::{
            get_identities_balances_response_v0::Result as V0Result,
            GetIdentitiesBalancesResponseV0, Version,
        };

        let response = platform::GetIdentitiesBalancesResponse {
            version: Some(Version::V0(GetIdentitiesBalancesResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };

        let request: platform::GetIdentitiesBalancesRequest = GetIdentitiesBalancesRequestV0 {
            ids: vec![vec![0u8; 10]], // wrong length
            prove: true,
        }
        .into();

        let provider = unreachable_provider();
        let err =
            <IdentityBalances as FromProof<platform::GetIdentitiesBalancesRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("all 32 bytes"), "got: {error}");
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn data_contracts_rejects_wrong_size_id() {
        use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
        use platform::get_data_contracts_response::{
            get_data_contracts_response_v0::Result as V0Result, GetDataContractsResponseV0, Version,
        };

        let response = platform::GetDataContractsResponse {
            version: Some(Version::V0(GetDataContractsResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetDataContractsRequest = GetDataContractsRequestV0 {
            ids: vec![vec![0u8; 20]], // must be 32 bytes
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <DataContracts as FromProof<platform::GetDataContractsRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("wrong id size"), "got: {error}");
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn upgrade_vote_status_rejects_bad_start_pro_tx_hash_length() {
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0;
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_response::{
            get_protocol_version_upgrade_vote_status_response_v0::Result as V0Result,
            GetProtocolVersionUpgradeVoteStatusResponseV0, Version,
        };

        let response = GetProtocolVersionUpgradeVoteStatusResponse {
            version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        // start_pro_tx_hash must be 32 bytes if non-empty.
        let request: GetProtocolVersionUpgradeVoteStatusRequest =
            GetProtocolVersionUpgradeVoteStatusRequestV0 {
                start_pro_tx_hash: vec![0u8; 5],
                count: 10,
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <MasternodeProtocolVotes as FromProof<
            GetProtocolVersionUpgradeVoteStatusRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { .. } => {}
            other => panic!("expected RequestError for bad pro_tx_hash length, got: {other:?}"),
        }
    }

    #[test]
    fn upgrade_vote_status_empty_version_on_request_none() {
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_response::{
            get_protocol_version_upgrade_vote_status_response_v0::Result as V0Result,
            GetProtocolVersionUpgradeVoteStatusResponseV0, Version,
        };
        let response = GetProtocolVersionUpgradeVoteStatusResponse {
            version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = GetProtocolVersionUpgradeVoteStatusRequest { version: None };
        let provider = unreachable_provider();
        let err = <MasternodeProtocolVotes as FromProof<
            GetProtocolVersionUpgradeVoteStatusRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn path_elements_no_proof_without_response() {
        let request = GetPathElementsRequest::default();
        let response = GetPathElementsResponse::default();
        let provider = unreachable_provider();
        let err = <Elements as FromProof<GetPathElementsRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn prefunded_balance_rejects_bad_id_length() {
        use dapi_grpc::platform::v0::get_prefunded_specialized_balance_request::GetPrefundedSpecializedBalanceRequestV0;
        use platform::get_prefunded_specialized_balance_response::{
            get_prefunded_specialized_balance_response_v0::Result as V0Result,
            GetPrefundedSpecializedBalanceResponseV0, Version,
        };
        let response = platform::GetPrefundedSpecializedBalanceResponse {
            version: Some(Version::V0(GetPrefundedSpecializedBalanceResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetPrefundedSpecializedBalanceRequest =
            GetPrefundedSpecializedBalanceRequestV0 {
                id: vec![0u8; 3], // must be 32
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <PrefundedSpecializedBalance as FromProof<
            platform::GetPrefundedSpecializedBalanceRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn epochs_info_rejects_overflowing_start_epoch() {
        use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
        use platform::get_epochs_info_response::{
            get_epochs_info_response_v0::Result as V0Result, GetEpochsInfoResponseV0, Version,
        };
        let response = platform::GetEpochsInfoResponse {
            version: Some(Version::V0(GetEpochsInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata {
                    epoch: 10,
                    ..Default::default()
                }),
            })),
        };
        // start_epoch > u16::MAX triggers try_u32_to_u16 error.
        let request = platform::GetEpochsInfoRequest {
            version: Some(platform::get_epochs_info_request::Version::V0(
                GetEpochsInfoRequestV0 {
                    start_epoch: Some(100_000),
                    count: 1,
                    ascending: true,
                    prove: true,
                },
            )),
        };
        let provider = unreachable_provider();
        let err =
            <ExtendedEpochInfos as FromProof<platform::GetEpochsInfoRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn broadcast_state_transition_rejects_garbage_payload() {
        // Cannot deserialize random bytes into a StateTransition, so we hit
        // the ProtocolError branch before any proof work happens.
        let request = platform::BroadcastStateTransitionRequest {
            state_transition: vec![0xFFu8; 16], // nonsense bytes
        };
        // Response structure only needs to have a valid proof field since
        // deserialize happens after proof extraction.
        use platform::wait_for_state_transition_result_response::{
            wait_for_state_transition_result_response_v0::Result as V0Result, Version,
            WaitForStateTransitionResultResponseV0,
        };
        let response = platform::WaitForStateTransitionResultResponse {
            version: Some(Version::V0(WaitForStateTransitionResultResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let provider = unreachable_provider();
        let err = <StateTransitionProofOutcome as FromProof<
            platform::BroadcastStateTransitionRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::ProtocolError { .. }),
            "expected ProtocolError from StateTransition decode, got: {err:?}"
        );
    }

    // ---------------------------------------------------------------------
    // Additional coverage: numeric helpers
    // ---------------------------------------------------------------------

    #[test]
    fn u32_to_u16_opt_zero_maps_to_none() {
        // zero -> None (not Some(0)): guards against accidentally treating
        // "unset" as "limit 0".
        let parsed = u32_to_u16_opt(0).unwrap();
        assert!(parsed.is_none(), "value 0 must decode to None");
    }

    #[test]
    fn u32_to_u16_opt_at_boundary() {
        let parsed = u32_to_u16_opt(u16::MAX as u32).unwrap();
        assert_eq!(parsed, Some(u16::MAX));
    }

    #[test]
    fn u32_to_u16_opt_error_just_above_boundary() {
        // u16::MAX + 1 is the minimal out-of-range value.
        let err = u32_to_u16_opt((u16::MAX as u32) + 1).unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("out of range"), "got: {error}");
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn try_u32_to_u16_at_boundary_plus_one() {
        // u16::MAX is ok; u16::MAX+1 is not.
        assert!(try_u32_to_u16(u16::MAX as u32).is_ok());
        assert!(try_u32_to_u16((u16::MAX as u32) + 1).is_err());
    }

    // ---------------------------------------------------------------------
    // Additional coverage: Length / IntoOption edge cases
    // ---------------------------------------------------------------------

    #[test]
    fn length_option_of_length_none_counts_zero() {
        let none_opt: Option<Vec<Option<u32>>> = None;
        assert_eq!(none_opt.count(), 0);
        assert_eq!(none_opt.count_some(), 0);
    }

    #[test]
    fn length_vec_of_key_option_pair_only_none_values() {
        let v: Vec<(u8, Option<u32>)> = vec![(1, None), (2, None)];
        assert_eq!(v.count(), 2);
        assert_eq!(
            v.count_some(),
            0,
            "count_some must only count entries whose value is Some"
        );
    }

    #[test]
    fn length_btreemap_only_none_values() {
        let mut m: BTreeMap<u8, Option<u32>> = BTreeMap::new();
        m.insert(1, None);
        m.insert(2, None);
        assert_eq!(m.count(), 2);
        assert_eq!(m.count_some(), 0);
    }

    #[test]
    fn into_option_for_vec_of_key_option_pair_empty_and_nonempty() {
        let empty: Vec<(u8, Option<u32>)> = vec![];
        assert!(
            empty.into_option().is_none(),
            "empty vec must decode to None"
        );

        let single: Vec<(u8, Option<u32>)> = vec![(1, None)];
        assert!(
            single.into_option().is_some(),
            "non-empty vec with only None values must still be Some"
        );
    }

    #[test]
    fn into_option_for_btreemap_empty_and_nonempty() {
        let empty: BTreeMap<u8, Option<u32>> = BTreeMap::new();
        assert!(empty.into_option().is_none());

        let mut m: BTreeMap<u8, Option<u32>> = BTreeMap::new();
        m.insert(1, None);
        assert!(m.into_option().is_some());
    }

    // ---------------------------------------------------------------------
    // Additional coverage: parse_key_request_type
    // ---------------------------------------------------------------------

    #[test]
    fn parse_key_request_type_specific_keys_empty_ids() {
        // Exercises the SpecificKeys branch when the id list is empty.
        use dapi_grpc::platform::v0::SpecificKeys;
        let outer = Some(GrpcKeyType {
            request: Some(key_request_type::Request::SpecificKeys(SpecificKeys {
                key_ids: vec![],
            })),
        });
        let parsed = parse_key_request_type(&outer).unwrap();
        match parsed {
            KeyRequestType::SpecificKeys(ids) => {
                assert!(ids.is_empty(), "empty ids must round-trip as empty");
            }
            _ => panic!("expected SpecificKeys variant"),
        }
    }

    #[test]
    fn parse_key_request_type_search_key_empty_purpose_map() {
        // Exercises the SearchKey branch when the purpose map is empty.
        use dapi_grpc::platform::v0::SearchKey;
        let outer = Some(GrpcKeyType {
            request: Some(key_request_type::Request::SearchKey(SearchKey {
                purpose_map: std::collections::HashMap::new(),
            })),
        });
        let parsed = parse_key_request_type(&outer).unwrap();
        match parsed {
            KeyRequestType::SearchKey(m) => {
                assert!(m.is_empty(), "empty map must round-trip as empty");
            }
            _ => panic!("expected SearchKey variant"),
        }
    }

    #[test]
    fn parse_key_request_type_search_key_negative_kind_rejected() {
        // Negative i32 values are not valid GrpcKeyKind values -> RequestError.
        use dapi_grpc::platform::v0::{SearchKey, SecurityLevelMap};
        let mut sec_map: std::collections::HashMap<u32, i32> = std::collections::HashMap::new();
        sec_map.insert(0, -1);
        let mut purpose_map = std::collections::HashMap::new();
        purpose_map.insert(
            0u32,
            SecurityLevelMap {
                security_level_map: sec_map,
            },
        );
        let outer = Some(GrpcKeyType {
            request: Some(key_request_type::Request::SearchKey(SearchKey {
                purpose_map,
            })),
        });
        // KeyRequestType does not implement Debug, so use `.err().expect(...)`
        // rather than `.unwrap_err()`.
        let err = parse_key_request_type(&outer)
            .err()
            .expect("negative kind must error");
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("missing requested key type"), "got: {error}");
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------
    // Additional coverage: FromProof wrapper methods
    // ---------------------------------------------------------------------

    /// FromProof impl that always succeeds with a Some value; used for
    /// exercising the `from_proof*` wrappers' happy paths.
    #[derive(Debug, PartialEq)]
    struct PresentFromProof(u32);

    impl FromProof<()> for PresentFromProof {
        type Request = ();
        type Response = ();

        fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
            _request: I,
            _response: O,
            _network: Network,
            _platform_version: &PlatformVersion,
            _provider: &'a dyn ContextProvider,
        ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
        where
            Self: Sized + 'a,
        {
            Ok((
                Some(PresentFromProof(7)),
                ResponseMetadata {
                    height: 123,
                    ..Default::default()
                },
                Proof::default(),
            ))
        }
    }

    #[test]
    fn from_proof_with_metadata_returns_value_and_metadata() {
        // Ensures the `from_proof_with_metadata` wrapper unwraps Some correctly.
        let provider = unreachable_provider();
        let (value, mtd) = <PresentFromProof as FromProof<()>>::from_proof_with_metadata(
            (),
            (),
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap();
        assert_eq!(value, PresentFromProof(7));
        assert_eq!(mtd.height, 123);
    }

    #[test]
    fn from_proof_with_metadata_and_proof_returns_all_three() {
        let provider = unreachable_provider();
        let (value, mtd, _proof) =
            <PresentFromProof as FromProof<()>>::from_proof_with_metadata_and_proof(
                (),
                (),
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap();
        assert_eq!(value, PresentFromProof(7));
        assert_eq!(mtd.height, 123);
    }

    #[test]
    fn from_proof_on_missing_returns_not_found_via_wrapper() {
        // `from_proof` forwards to `maybe_from_proof` then maps None -> NotFound.
        let provider = unreachable_provider();
        let err = <MissingFromProof as FromProof<()>>::from_proof_with_metadata(
            (),
            (),
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NotFound), "got: {err:?}");
    }

    #[test]
    fn from_proof_with_metadata_and_proof_missing_returns_not_found() {
        let provider = unreachable_provider();
        let err = <MissingFromProof as FromProof<()>>::from_proof_with_metadata_and_proof(
            (),
            (),
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NotFound), "got: {err:?}");
    }

    #[test]
    fn maybe_from_proof_delegates_to_with_metadata_and_forwards_none() {
        // `maybe_from_proof` discards metadata/proof when forwarding the
        // underlying `(None, _, _)` shape.
        let provider = unreachable_provider();
        let result = <MissingFromProof as FromProof<()>>::maybe_from_proof(
            (),
            (),
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap();
        assert!(result.is_none(), "MissingFromProof must bubble None");
    }

    // ---------------------------------------------------------------------
    // Additional coverage: more FromProof impls' decode-error paths
    // ---------------------------------------------------------------------

    fn default_metadata_with_epoch(epoch: u32) -> ResponseMetadata {
        ResponseMetadata {
            epoch,
            ..Default::default()
        }
    }

    #[test]
    fn identity_keys_rejects_bad_identity_id_length() {
        use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
        use platform::get_identity_keys_response::{
            get_identity_keys_response_v0::Result as V0Result, GetIdentityKeysResponseV0, Version,
        };

        let response = platform::GetIdentityKeysResponse {
            version: Some(Version::V0(GetIdentityKeysResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityKeysRequest = GetIdentityKeysRequestV0 {
            identity_id: vec![0u8; 5], // must be 32
            request_type: None,
            limit: None,
            offset: None,
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <IdentityPublicKeys as FromProof<platform::GetIdentityKeysRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identity_keys_rejects_overflowing_limit() {
        use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
        use platform::get_identity_keys_response::{
            get_identity_keys_response_v0::Result as V0Result, GetIdentityKeysResponseV0, Version,
        };

        let response = platform::GetIdentityKeysResponse {
            version: Some(Version::V0(GetIdentityKeysResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityKeysRequest = GetIdentityKeysRequestV0 {
            identity_id: vec![0u8; 32], // valid
            request_type: None,
            limit: Some(100_000),
            offset: None,
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <IdentityPublicKeys as FromProof<platform::GetIdentityKeysRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn identity_keys_rejects_overflowing_offset() {
        use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
        use platform::get_identity_keys_response::{
            get_identity_keys_response_v0::Result as V0Result, GetIdentityKeysResponseV0, Version,
        };

        let response = platform::GetIdentityKeysResponse {
            version: Some(Version::V0(GetIdentityKeysResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityKeysRequest = GetIdentityKeysRequestV0 {
            identity_id: vec![0u8; 32],
            request_type: None,
            limit: None,
            offset: Some(100_000),
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <IdentityPublicKeys as FromProof<platform::GetIdentityKeysRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn identity_keys_rejects_missing_key_request_type() {
        // limit/offset are valid and identity_id is valid, so execution
        // reaches parse_key_request_type which errors on None.
        use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
        use platform::get_identity_keys_response::{
            get_identity_keys_response_v0::Result as V0Result, GetIdentityKeysResponseV0, Version,
        };

        let response = platform::GetIdentityKeysResponse {
            version: Some(Version::V0(GetIdentityKeysResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityKeysRequest = GetIdentityKeysRequestV0 {
            identity_id: vec![0u8; 32],
            request_type: None,
            limit: None,
            offset: None,
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <IdentityPublicKeys as FromProof<platform::GetIdentityKeysRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("missing key request type"), "got: {error}");
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn identity_keys_no_proof_when_response_empty() {
        let request = platform::GetIdentityKeysRequest::default();
        let response = platform::GetIdentityKeysResponse::default();
        let provider = unreachable_provider();
        let err =
            <IdentityPublicKeys as FromProof<platform::GetIdentityKeysRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn identity_nonce_rejects_bad_identity_id_length() {
        use dapi_grpc::platform::v0::get_identity_nonce_request::GetIdentityNonceRequestV0;
        use platform::get_identity_nonce_response::{
            get_identity_nonce_response_v0::Result as V0Result, GetIdentityNonceResponseV0, Version,
        };

        let response = platform::GetIdentityNonceResponse {
            version: Some(Version::V0(GetIdentityNonceResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityNonceRequest = GetIdentityNonceRequestV0 {
            identity_id: vec![0u8; 1], // must be 32
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err = <IdentityNonceFetcher as FromProof<
            platform::GetIdentityNonceRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identity_nonce_no_proof_when_response_empty() {
        let request = platform::GetIdentityNonceRequest::default();
        let response = platform::GetIdentityNonceResponse::default();
        let provider = unreachable_provider();
        let err = <IdentityNonceFetcher as FromProof<
            platform::GetIdentityNonceRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn identity_contract_nonce_rejects_bad_identity_id_length() {
        use dapi_grpc::platform::v0::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0;
        use platform::get_identity_contract_nonce_response::{
            get_identity_contract_nonce_response_v0::Result as V0Result,
            GetIdentityContractNonceResponseV0, Version,
        };

        let response = platform::GetIdentityContractNonceResponse {
            version: Some(Version::V0(GetIdentityContractNonceResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityContractNonceRequest =
            GetIdentityContractNonceRequestV0 {
                identity_id: vec![0u8; 10], // must be 32
                contract_id: vec![0u8; 32],
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <IdentityContractNonceFetcher as FromProof<
            platform::GetIdentityContractNonceRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identity_contract_nonce_rejects_bad_contract_id_length() {
        use dapi_grpc::platform::v0::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0;
        use platform::get_identity_contract_nonce_response::{
            get_identity_contract_nonce_response_v0::Result as V0Result,
            GetIdentityContractNonceResponseV0, Version,
        };

        let response = platform::GetIdentityContractNonceResponse {
            version: Some(Version::V0(GetIdentityContractNonceResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityContractNonceRequest =
            GetIdentityContractNonceRequestV0 {
                identity_id: vec![0u8; 32],
                contract_id: vec![0u8; 10], // must be 32
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <IdentityContractNonceFetcher as FromProof<
            platform::GetIdentityContractNonceRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identity_balance_rejects_bad_id_length() {
        use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
        use platform::get_identity_balance_response::{
            get_identity_balance_response_v0::Result as V0Result, GetIdentityBalanceResponseV0,
            Version,
        };

        let response = platform::GetIdentityBalanceResponse {
            version: Some(Version::V0(GetIdentityBalanceResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityBalanceRequest = GetIdentityBalanceRequestV0 {
            id: vec![0u8; 5],
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <IdentityBalance as FromProof<platform::GetIdentityBalanceRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identity_balance_empty_version_on_request_version_none() {
        // response OK; request.version None -> EmptyVersion.
        use platform::get_identity_balance_response::{
            get_identity_balance_response_v0::Result as V0Result, GetIdentityBalanceResponseV0,
            Version,
        };
        let response = platform::GetIdentityBalanceResponse {
            version: Some(Version::V0(GetIdentityBalanceResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetIdentityBalanceRequest { version: None };
        let provider = unreachable_provider();
        let err =
            <IdentityBalance as FromProof<platform::GetIdentityBalanceRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn identity_balance_and_revision_rejects_bad_id_length() {
        use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0;
        use platform::get_identity_balance_and_revision_response::{
            get_identity_balance_and_revision_response_v0::Result as V0Result,
            GetIdentityBalanceAndRevisionResponseV0, Version,
        };
        let response = platform::GetIdentityBalanceAndRevisionResponse {
            version: Some(Version::V0(GetIdentityBalanceAndRevisionResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentityBalanceAndRevisionRequest =
            GetIdentityBalanceAndRevisionRequestV0 {
                id: vec![0u8; 5],
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <IdentityBalanceAndRevision as FromProof<
            platform::GetIdentityBalanceAndRevisionRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identities_balances_empty_version_none() {
        use platform::get_identities_balances_response::{
            get_identities_balances_response_v0::Result as V0Result,
            GetIdentitiesBalancesResponseV0, Version,
        };
        let response = platform::GetIdentitiesBalancesResponse {
            version: Some(Version::V0(GetIdentitiesBalancesResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetIdentitiesBalancesRequest { version: None };
        let provider = unreachable_provider();
        let err =
            <IdentityBalances as FromProof<platform::GetIdentitiesBalancesRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn data_contract_rejects_bad_id_length() {
        use dapi_grpc::platform::v0::get_data_contract_request::GetDataContractRequestV0;
        use platform::get_data_contract_response::{
            get_data_contract_response_v0::Result as V0Result, GetDataContractResponseV0, Version,
        };
        let response = platform::GetDataContractResponse {
            version: Some(Version::V0(GetDataContractResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetDataContractRequest = GetDataContractRequestV0 {
            id: vec![0u8; 5],
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err = <DataContract as FromProof<platform::GetDataContractRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn data_contract_no_proof_when_response_empty() {
        let request = platform::GetDataContractRequest::default();
        let response = platform::GetDataContractResponse::default();
        let provider = unreachable_provider();
        let err = <DataContract as FromProof<platform::GetDataContractRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn data_contract_with_serialization_rejects_bad_id_length() {
        // This hits the second `FromProof for (DataContract, Vec<u8>)` impl.
        use dapi_grpc::platform::v0::get_data_contract_request::GetDataContractRequestV0;
        use platform::get_data_contract_response::{
            get_data_contract_response_v0::Result as V0Result, GetDataContractResponseV0, Version,
        };
        let response = platform::GetDataContractResponse {
            version: Some(Version::V0(GetDataContractResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetDataContractRequest = GetDataContractRequestV0 {
            id: vec![0u8; 5],
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <(DataContract, Vec<u8>) as FromProof<platform::GetDataContractRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn data_contract_history_rejects_bad_id_length() {
        use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
        use platform::get_data_contract_history_response::{
            get_data_contract_history_response_v0::Result as V0Result,
            GetDataContractHistoryResponseV0, Version,
        };
        let response = platform::GetDataContractHistoryResponse {
            version: Some(Version::V0(GetDataContractHistoryResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetDataContractHistoryRequest = GetDataContractHistoryRequestV0 {
            id: vec![0u8; 5],
            limit: None,
            offset: None,
            start_at_ms: 0,
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err = <DataContractHistory as FromProof<
            platform::GetDataContractHistoryRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn data_contract_history_rejects_overflowing_limit() {
        use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
        use platform::get_data_contract_history_response::{
            get_data_contract_history_response_v0::Result as V0Result,
            GetDataContractHistoryResponseV0, Version,
        };
        let response = platform::GetDataContractHistoryResponse {
            version: Some(Version::V0(GetDataContractHistoryResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetDataContractHistoryRequest = GetDataContractHistoryRequestV0 {
            id: vec![0u8; 32],
            limit: Some(100_000),
            offset: None,
            start_at_ms: 0,
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err = <DataContractHistory as FromProof<
            platform::GetDataContractHistoryRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn document_history_no_proof_when_response_empty() {
        let request = platform::GetDocumentHistoryRequest::default();
        let response = platform::GetDocumentHistoryResponse::default();
        let provider = unreachable_provider();

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn document_history_empty_response_metadata() {
        use platform::get_document_history_response::{
            get_document_history_response_v0::Result as V0Result, GetDocumentHistoryResponseV0,
            Version,
        };
        let request = platform::GetDocumentHistoryRequest::default();
        let response = platform::GetDocumentHistoryResponse {
            version: Some(Version::V0(GetDocumentHistoryResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: None,
            })),
        };
        let provider = unreachable_provider();

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }

    #[test]
    fn document_history_empty_version() {
        let request = platform::GetDocumentHistoryRequest { version: None };
        let response = document_history_response_with_proof_and_metadata();
        let provider = unreachable_provider();

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn document_history_rejects_bad_contract_id_length() {
        let request =
            document_history_request(vec![0u8; 5], "niceDocument", vec![1u8; 32], None, None);
        let response = document_history_response_with_proof_and_metadata();
        let provider = unreachable_provider();

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn document_history_rejects_bad_document_id_length() {
        let request =
            document_history_request(vec![0u8; 32], "niceDocument", vec![1u8; 5], None, None);
        let response = document_history_response_with_proof_and_metadata();
        let provider = unreachable_provider();

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn document_history_rejects_overflowing_limit() {
        let request = document_history_request(
            vec![0u8; 32],
            "niceDocument",
            vec![1u8; 32],
            Some(100_000),
            None,
        );
        let response = document_history_response_with_proof_and_metadata();
        let provider = unreachable_provider();

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn document_history_returns_not_found_when_contract_provider_misses() {
        let request =
            document_history_request(vec![0u8; 32], "niceDocument", vec![1u8; 32], None, None);
        let response = document_history_response_with_proof_and_metadata();
        let provider = NoDataContractProvider;

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::NotFound), "got: {err:?}");
    }

    #[test]
    fn document_history_rejects_unknown_document_type() {
        let data_contract = dpp::tests::fixtures::get_data_contract_fixture(
            None,
            0,
            default_platform_version().protocol_version,
        )
        .data_contract_owned();
        let contract_id = data_contract.id().to_vec();
        let provider = StaticDataContractProvider {
            data_contract: std::sync::Arc::new(data_contract),
        };
        let request =
            document_history_request(contract_id, "missingDocument", vec![1u8; 32], None, None);
        let response = document_history_response_with_proof_and_metadata();

        let err = document_history_error(request, response, &provider);

        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn address_info_rejects_bad_address_bytes() {
        // PlatformAddress::from_bytes fails for invalid lengths.
        use dapi_grpc::platform::v0::get_address_info_request::GetAddressInfoRequestV0;
        use platform::get_address_info_response::{
            get_address_info_response_v0::Result as V0Result, GetAddressInfoResponseV0, Version,
        };
        let response = platform::GetAddressInfoResponse {
            version: Some(Version::V0(GetAddressInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetAddressInfoRequest = GetAddressInfoRequestV0 {
            address: vec![0u8; 3], // not a valid address length
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err = <AddressInfo as FromProof<platform::GetAddressInfoRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn addresses_infos_rejects_bad_address_bytes() {
        use dapi_grpc::platform::v0::get_addresses_infos_request::GetAddressesInfosRequestV0;
        use platform::get_addresses_infos_response::{
            get_addresses_infos_response_v0::Result as V0Result, GetAddressesInfosResponseV0,
            Version,
        };
        let response = platform::GetAddressesInfosResponse {
            version: Some(Version::V0(GetAddressesInfosResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetAddressesInfosRequest = GetAddressesInfosRequestV0 {
            addresses: vec![vec![0u8; 3]],
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err =
            <AddressInfos as FromProof<platform::GetAddressesInfosRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn addresses_trunk_state_grove_no_proof() {
        // GroveTrunkQueryResult impl ignores the request entirely, so the
        // first failure possible is NoProofInResult when the response is empty.
        let response = platform::GetAddressesTrunkStateResponse::default();
        let request = platform::GetAddressesTrunkStateRequest::default();
        let provider = unreachable_provider();
        let err = <GroveTrunkQueryResult as FromProof<
            platform::GetAddressesTrunkStateRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn platform_address_trunk_state_no_proof() {
        // PlatformAddressTrunkState wraps the GroveTrunkQueryResult impl; same error.
        let response = platform::GetAddressesTrunkStateResponse::default();
        let request = platform::GetAddressesTrunkStateRequest::default();
        let provider = unreachable_provider();
        let err = <PlatformAddressTrunkState as FromProof<
            platform::GetAddressesTrunkStateRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn epochs_info_empty_version_none() {
        use platform::get_epochs_info_response::{
            get_epochs_info_response_v0::Result as V0Result, GetEpochsInfoResponseV0, Version,
        };
        let response = platform::GetEpochsInfoResponse {
            version: Some(Version::V0(GetEpochsInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetEpochsInfoRequest { version: None };
        let provider = unreachable_provider();
        let err =
            <ExtendedEpochInfos as FromProof<platform::GetEpochsInfoRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn epochs_info_rejects_overflowing_count() {
        // start_epoch valid, but count > u16::MAX -> try_u32_to_u16 errors.
        use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
        use platform::get_epochs_info_response::{
            get_epochs_info_response_v0::Result as V0Result, GetEpochsInfoResponseV0, Version,
        };
        let response = platform::GetEpochsInfoResponse {
            version: Some(Version::V0(GetEpochsInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(10)),
            })),
        };
        let request = platform::GetEpochsInfoRequest {
            version: Some(platform::get_epochs_info_request::Version::V0(
                GetEpochsInfoRequestV0 {
                    start_epoch: Some(0),
                    count: 100_000,
                    ascending: true,
                    prove: true,
                },
            )),
        };
        let provider = unreachable_provider();
        let err =
            <ExtendedEpochInfos as FromProof<platform::GetEpochsInfoRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn epochs_info_ascending_without_start_ignores_metadata_epoch() {
        // Ascending queries without a start are request-derived from epoch 0,
        // so unsigned metadata must not participate in query selection.
        use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
        use platform::get_epochs_info_response::{
            get_epochs_info_response_v0::Result as V0Result, GetEpochsInfoResponseV0, Version,
        };
        let response = platform::GetEpochsInfoResponse {
            version: Some(Version::V0(GetEpochsInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(70_000)),
            })),
        };
        let request = platform::GetEpochsInfoRequest {
            version: Some(platform::get_epochs_info_request::Version::V0(
                GetEpochsInfoRequestV0 {
                    start_epoch: None,
                    count: 1,
                    ascending: true,
                    prove: true,
                },
            )),
        };
        let provider = unreachable_provider();
        let err =
            <ExtendedEpochInfos as FromProof<platform::GetEpochsInfoRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(
            !matches!(err, Error::RequestError { .. }),
            "metadata epoch unexpectedly influenced the request: {err:?}"
        );
    }

    #[test]
    fn epochs_info_descending_requires_explicit_start_epoch() {
        use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
        use platform::get_epochs_info_response::{
            get_epochs_info_response_v0::Result as V0Result, GetEpochsInfoResponseV0, Version,
        };
        let response = platform::GetEpochsInfoResponse {
            version: Some(Version::V0(GetEpochsInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(10)),
            })),
        };
        let request = platform::GetEpochsInfoRequest {
            version: Some(platform::get_epochs_info_request::Version::V0(
                GetEpochsInfoRequestV0 {
                    start_epoch: None,
                    count: 1,
                    ascending: false,
                    prove: true,
                },
            )),
        };
        let provider = unreachable_provider();

        let err =
            <ExtendedEpochInfos as FromProof<platform::GetEpochsInfoRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();

        match err {
            Error::RequestError { error } => assert!(error.contains("explicit start epoch")),
            other => panic!("expected explicit-epoch request error, got: {other:?}"),
        }
    }

    #[test]
    fn epochs_info_descending_with_explicit_max_start_passes_guard() {
        // The SDK's `fetch_current` sends a descending query with an explicit
        // start (the hinted current epoch index). Any explicit start — up to
        // MAX_EPOCH, the highest index that fits Drive's epoch key encoding —
        // makes the request fully self-describing, so it must get past the
        // explicit-start guard and proceed to proof verification (which here
        // fails on the garbage proof, not on the request shape).
        use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
        use dpp::block::epoch::MAX_EPOCH;
        use platform::get_epochs_info_response::{
            get_epochs_info_response_v0::Result as V0Result, GetEpochsInfoResponseV0, Version,
        };
        let response = platform::GetEpochsInfoResponse {
            version: Some(Version::V0(GetEpochsInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(10)),
            })),
        };
        let request = platform::GetEpochsInfoRequest {
            version: Some(platform::get_epochs_info_request::Version::V0(
                GetEpochsInfoRequestV0 {
                    start_epoch: Some(MAX_EPOCH as u32),
                    count: 1,
                    ascending: false,
                    prove: true,
                },
            )),
        };
        let provider = unreachable_provider();

        let err =
            <ExtendedEpochInfos as FromProof<platform::GetEpochsInfoRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();

        assert!(
            !matches!(err, Error::RequestError { .. }),
            "explicit-start descending query must pass the guard, got: {err:?}"
        );
    }

    #[test]
    fn extended_epoch_info_single_bubbles_empty_version() {
        // Ensures the wrapper passes through error from inner impl.
        use platform::get_epochs_info_response::{
            get_epochs_info_response_v0::Result as V0Result, GetEpochsInfoResponseV0, Version,
        };
        let response = platform::GetEpochsInfoResponse {
            version: Some(Version::V0(GetEpochsInfoResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetEpochsInfoRequest { version: None };
        let provider = unreachable_provider();
        let err =
            <ExtendedEpochInfo as FromProof<platform::GetEpochsInfoRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn finalized_epoch_infos_rejects_overflowing_start_index() {
        use dapi_grpc::platform::v0::get_finalized_epoch_infos_request::GetFinalizedEpochInfosRequestV0;
        use platform::get_finalized_epoch_infos_response::{
            get_finalized_epoch_infos_response_v0::Result as V0Result,
            GetFinalizedEpochInfosResponseV0, Version,
        };
        let response = platform::GetFinalizedEpochInfosResponse {
            version: Some(Version::V0(GetFinalizedEpochInfosResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetFinalizedEpochInfosRequest = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 100_000,
            start_epoch_index_included: true,
            end_epoch_index: 1,
            end_epoch_index_included: true,
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err = <FinalizedEpochInfos as FromProof<
            platform::GetFinalizedEpochInfosRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn finalized_epoch_infos_rejects_overflowing_end_index() {
        use dapi_grpc::platform::v0::get_finalized_epoch_infos_request::GetFinalizedEpochInfosRequestV0;
        use platform::get_finalized_epoch_infos_response::{
            get_finalized_epoch_infos_response_v0::Result as V0Result,
            GetFinalizedEpochInfosResponseV0, Version,
        };
        let response = platform::GetFinalizedEpochInfosResponse {
            version: Some(Version::V0(GetFinalizedEpochInfosResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetFinalizedEpochInfosRequest = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 1,
            start_epoch_index_included: true,
            end_epoch_index: 100_000,
            end_epoch_index_included: true,
            prove: true,
        }
        .into();
        let provider = unreachable_provider();
        let err = <FinalizedEpochInfos as FromProof<
            platform::GetFinalizedEpochInfosRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn upgrade_state_no_proof_when_response_empty() {
        // No request dependency, so the first error must come from the response.
        let response = GetProtocolVersionUpgradeStateResponse::default();
        let request = GetProtocolVersionUpgradeStateRequest::default();
        let provider = unreachable_provider();
        let err = <ProtocolVersionUpgrades as FromProof<
            GetProtocolVersionUpgradeStateRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn upgrade_vote_status_no_proof_when_response_empty() {
        let response = GetProtocolVersionUpgradeVoteStatusResponse::default();
        let request = GetProtocolVersionUpgradeVoteStatusRequest::default();
        let provider = unreachable_provider();
        let err = <MasternodeProtocolVotes as FromProof<
            GetProtocolVersionUpgradeVoteStatusRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn upgrade_vote_status_rejects_overflowing_count() {
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0;
        use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_response::{
            get_protocol_version_upgrade_vote_status_response_v0::Result as V0Result,
            GetProtocolVersionUpgradeVoteStatusResponseV0, Version,
        };

        let response = GetProtocolVersionUpgradeVoteStatusResponse {
            version: Some(Version::V0(GetProtocolVersionUpgradeVoteStatusResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        // empty start_pro_tx_hash (None branch), but count overflow
        let request: GetProtocolVersionUpgradeVoteStatusRequest =
            GetProtocolVersionUpgradeVoteStatusRequestV0 {
                start_pro_tx_hash: vec![],
                count: 100_000,
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <MasternodeProtocolVotes as FromProof<
            GetProtocolVersionUpgradeVoteStatusRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn path_elements_empty_version_when_no_version() {
        // Response has full v0 shell, request has None -> EmptyVersion.
        use platform::get_path_elements_response::{
            get_path_elements_response_v0::Result as V0Result, GetPathElementsResponseV0, Version,
        };
        let response = GetPathElementsResponse {
            version: Some(Version::V0(GetPathElementsResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = GetPathElementsRequest { version: None };
        let provider = unreachable_provider();
        let err = <Elements as FromProof<GetPathElementsRequest>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn identities_contract_keys_rejects_bad_identity_id() {
        use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
        use platform::get_identities_contract_keys_response::{
            get_identities_contract_keys_response_v0::Result as V0Result,
            GetIdentitiesContractKeysResponseV0, Version,
        };
        let response = platform::GetIdentitiesContractKeysResponse {
            version: Some(Version::V0(GetIdentitiesContractKeysResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentitiesContractKeysRequest =
            GetIdentitiesContractKeysRequestV0 {
                identities_ids: vec![vec![0u8; 5]], // bad
                contract_id: vec![0u8; 32],
                document_type_name: None,
                purposes: vec![0],
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <IdentitiesContractKeys as FromProof<
            platform::GetIdentitiesContractKeysRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identities_contract_keys_rejects_bad_contract_id() {
        use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
        use platform::get_identities_contract_keys_response::{
            get_identities_contract_keys_response_v0::Result as V0Result,
            GetIdentitiesContractKeysResponseV0, Version,
        };
        let response = platform::GetIdentitiesContractKeysResponse {
            version: Some(Version::V0(GetIdentitiesContractKeysResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request: platform::GetIdentitiesContractKeysRequest =
            GetIdentitiesContractKeysRequestV0 {
                identities_ids: vec![vec![0u8; 32]],
                contract_id: vec![0u8; 5], // bad
                document_type_name: None,
                purposes: vec![0],
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <IdentitiesContractKeys as FromProof<
            platform::GetIdentitiesContractKeysRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn identities_contract_keys_rejects_bad_purpose() {
        use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
        use platform::get_identities_contract_keys_response::{
            get_identities_contract_keys_response_v0::Result as V0Result,
            GetIdentitiesContractKeysResponseV0, Version,
        };
        let response = platform::GetIdentitiesContractKeysResponse {
            version: Some(Version::V0(GetIdentitiesContractKeysResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        // purpose 250 is not a valid Purpose value.
        let request: platform::GetIdentitiesContractKeysRequest =
            GetIdentitiesContractKeysRequestV0 {
                identities_ids: vec![vec![0u8; 32]],
                contract_id: vec![0u8; 32],
                document_type_name: None,
                purposes: vec![250],
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <IdentitiesContractKeys as FromProof<
            platform::GetIdentitiesContractKeysRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }

    #[test]
    fn prefunded_balance_empty_version_on_request_version_none() {
        // response has proof; request.version = None -> EmptyVersion
        use platform::get_prefunded_specialized_balance_response::{
            get_prefunded_specialized_balance_response_v0::Result as V0Result,
            GetPrefundedSpecializedBalanceResponseV0, Version,
        };
        let response = platform::GetPrefundedSpecializedBalanceResponse {
            version: Some(Version::V0(GetPrefundedSpecializedBalanceResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetPrefundedSpecializedBalanceRequest { version: None };
        let provider = unreachable_provider();
        let err = <PrefundedSpecializedBalance as FromProof<
            platform::GetPrefundedSpecializedBalanceRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_ids_empty_version() {
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetEvonodesProposedEpochBlocksByIdsRequest { version: None };
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByIdsRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_ids_rejects_overflowing_epoch() {
        // Request sets epoch > u16::MAX -> try_u32_to_u16 error.
        use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::GetEvonodesProposedEpochBlocksByIdsRequestV0;
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(1)),
            })),
        };
        let request: platform::GetEvonodesProposedEpochBlocksByIdsRequest =
            GetEvonodesProposedEpochBlocksByIdsRequestV0 {
                epoch: Some(100_000),
                ids: vec![],
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByIdsRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_ids_requires_explicit_epoch() {
        // An omitted epoch must fail before unsigned metadata can select the
        // proof query.
        use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::GetEvonodesProposedEpochBlocksByIdsRequestV0;
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(99_999)),
            })),
        };
        let request: platform::GetEvonodesProposedEpochBlocksByIdsRequest =
            GetEvonodesProposedEpochBlocksByIdsRequestV0 {
                epoch: None,
                ids: vec![],
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByIdsRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_range_empty_version() {
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetEvonodesProposedEpochBlocksByRangeRequest { version: None };
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByRangeRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_range_requires_explicit_epoch() {
        use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::GetEvonodesProposedEpochBlocksByRangeRequestV0;
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(99_999)),
            })),
        };
        let request: platform::GetEvonodesProposedEpochBlocksByRangeRequest =
            GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: None,
                limit: None,
                start: None,
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByRangeRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();

        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_range_rejects_bad_start_after() {
        // Start::StartAfter with wrong length must fail.
        use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::{
            get_evonodes_proposed_epoch_blocks_by_range_request_v0::Start,
            GetEvonodesProposedEpochBlocksByRangeRequestV0,
        };
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(1)),
            })),
        };
        let request: platform::GetEvonodesProposedEpochBlocksByRangeRequest =
            GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(1),
                limit: None,
                start: Some(Start::StartAfter(vec![0u8; 5])),
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByRangeRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::DriveError { .. }), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_range_rejects_bad_start_at() {
        // Start::StartAt with wrong length must fail.
        use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::{
            get_evonodes_proposed_epoch_blocks_by_range_request_v0::Start,
            GetEvonodesProposedEpochBlocksByRangeRequestV0,
        };
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(1)),
            })),
        };
        let request: platform::GetEvonodesProposedEpochBlocksByRangeRequest =
            GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(1),
                limit: None,
                start: Some(Start::StartAt(vec![0u8; 4])),
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByRangeRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::DriveError { .. }), "got: {err:?}");
    }

    #[test]
    fn evonodes_proposed_epoch_blocks_by_range_rejects_overflow_limit() {
        use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::GetEvonodesProposedEpochBlocksByRangeRequestV0;
        use platform::get_evonodes_proposed_epoch_blocks_response::{
            get_evonodes_proposed_epoch_blocks_response_v0::Result as V0Result,
            GetEvonodesProposedEpochBlocksResponseV0, Version,
        };
        let response = platform::GetEvonodesProposedEpochBlocksResponse {
            version: Some(Version::V0(GetEvonodesProposedEpochBlocksResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(default_metadata_with_epoch(1)),
            })),
        };
        let request: platform::GetEvonodesProposedEpochBlocksByRangeRequest =
            GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(1),
                limit: Some(100_000),
                start: None,
                prove: true,
            }
            .into();
        let provider = unreachable_provider();
        let err = <ProposerBlockCounts as FromProof<
            platform::GetEvonodesProposedEpochBlocksByRangeRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn shielded_pool_state_no_proof_when_response_empty() {
        let response = platform::GetShieldedPoolStateResponse::default();
        let request = platform::GetShieldedPoolStateRequest::default();
        let provider = unreachable_provider();
        let err = <ShieldedPoolState as FromProof<
            platform::GetShieldedPoolStateRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn shielded_notes_count_no_proof_when_response_empty() {
        let response = platform::GetShieldedNotesCountResponse::default();
        let request = platform::GetShieldedNotesCountRequest::default();
        let provider = unreachable_provider();
        let err = <ShieldedNotesCount as FromProof<
            platform::GetShieldedNotesCountRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn shielded_anchors_no_proof_when_response_empty() {
        let response = platform::GetShieldedAnchorsResponse::default();
        let request = platform::GetShieldedAnchorsRequest::default();
        let provider = unreachable_provider();
        let err =
            <ShieldedAnchors as FromProof<platform::GetShieldedAnchorsRequest>>::maybe_from_proof(
                request,
                response,
                Network::Testnet,
                default_platform_version(),
                &provider,
            )
            .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn most_recent_shielded_anchor_no_proof_when_response_empty() {
        let response = platform::GetMostRecentShieldedAnchorResponse::default();
        let request = platform::GetMostRecentShieldedAnchorRequest::default();
        let provider = unreachable_provider();
        let err = <MostRecentShieldedAnchor as FromProof<
            platform::GetMostRecentShieldedAnchorRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn shielded_encrypted_notes_empty_version_on_request_version_none() {
        use platform::get_shielded_encrypted_notes_response::{
            get_shielded_encrypted_notes_response_v0::Result as V0Result,
            GetShieldedEncryptedNotesResponseV0, Version,
        };
        let response = platform::GetShieldedEncryptedNotesResponse {
            version: Some(Version::V0(GetShieldedEncryptedNotesResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetShieldedEncryptedNotesRequest { version: None };
        let provider = unreachable_provider();
        let err = <ShieldedEncryptedNotes as FromProof<
            platform::GetShieldedEncryptedNotesRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn shielded_encrypted_notes_no_proof_when_response_empty() {
        let response = platform::GetShieldedEncryptedNotesResponse::default();
        let request = platform::GetShieldedEncryptedNotesRequest::default();
        let provider = unreachable_provider();
        let err = <ShieldedEncryptedNotes as FromProof<
            platform::GetShieldedEncryptedNotesRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn shielded_nullifiers_empty_version_on_request_version_none() {
        use platform::get_shielded_nullifiers_response::{
            get_shielded_nullifiers_response_v0::Result as V0Result,
            GetShieldedNullifiersResponseV0, Version,
        };
        let response = platform::GetShieldedNullifiersResponse {
            version: Some(Version::V0(GetShieldedNullifiersResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetShieldedNullifiersRequest { version: None };
        let provider = unreachable_provider();
        let err = <ShieldedNullifierStatuses as FromProof<
            platform::GetShieldedNullifiersRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn recent_address_balance_changes_empty_version() {
        use platform::get_recent_address_balance_changes_response::{
            get_recent_address_balance_changes_response_v0::Result as V0Result,
            GetRecentAddressBalanceChangesResponseV0, Version,
        };
        let response = platform::GetRecentAddressBalanceChangesResponse {
            version: Some(Version::V0(GetRecentAddressBalanceChangesResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let request = platform::GetRecentAddressBalanceChangesRequest { version: None };
        let provider = unreachable_provider();
        let err = <RecentAddressBalanceChanges as FromProof<
            platform::GetRecentAddressBalanceChangesRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn recent_compacted_address_balance_changes_empty_version() {
        use platform::get_recent_compacted_address_balance_changes_response::{
            get_recent_compacted_address_balance_changes_response_v0::Result as V0Result,
            GetRecentCompactedAddressBalanceChangesResponseV0, Version,
        };
        let response = platform::GetRecentCompactedAddressBalanceChangesResponse {
            version: Some(Version::V0(
                GetRecentCompactedAddressBalanceChangesResponseV0 {
                    result: Some(V0Result::Proof(Proof::default())),
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        };
        let request = platform::GetRecentCompactedAddressBalanceChangesRequest { version: None };
        let provider = unreachable_provider();
        let err = <RecentCompactedAddressBalanceChanges as FromProof<
            platform::GetRecentCompactedAddressBalanceChangesRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn vote_identity_vote_rejects_bad_identity_id_length() {
        // The Vote impl validates id_in_request length before delegating.
        use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
        let request: platform::GetContestedResourceIdentityVotesRequest =
            GetContestedResourceIdentityVotesRequestV0 {
                identity_id: vec![0u8; 5], // bad
                limit: None,
                offset: None,
                order_ascending: true,
                start_at_vote_poll_id_info: None,
                prove: true,
            }
            .into();
        let response = platform::GetContestedResourceIdentityVotesResponse::default();
        let provider = unreachable_provider();
        let err = <Vote as FromProof<
            platform::GetContestedResourceIdentityVotesRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn vote_identity_vote_empty_version_on_request_none() {
        let request = platform::GetContestedResourceIdentityVotesRequest { version: None };
        let response = platform::GetContestedResourceIdentityVotesResponse::default();
        let provider = unreachable_provider();
        let err = <Vote as FromProof<
            platform::GetContestedResourceIdentityVotesRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn resource_votes_by_identity_empty_version_via_try_from_request() {
        // Delegates to ContestedResourceVotesGivenByIdentityQuery::try_from_request,
        // which returns Error::EmptyVersion when the request version is missing.
        let request = platform::GetContestedResourceIdentityVotesRequest { version: None };
        let response = platform::GetContestedResourceIdentityVotesResponse::default();
        let provider = unreachable_provider();
        let err = <ResourceVotesByIdentity as FromProof<
            platform::GetContestedResourceIdentityVotesRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    // ---------------------------------------------------------------------
    // IntoOption for more types
    // ---------------------------------------------------------------------

    #[test]
    fn into_option_indexmap_empty_vs_with_entry_only_none() {
        use dpp::prelude::Identifier;
        let empty: RetrievedObjects<Identifier, u32> = RetrievedObjects::new();
        assert!(empty.into_option().is_none());

        let mut map: RetrievedObjects<Identifier, u32> = RetrievedObjects::new();
        map.insert(Identifier::new([7u8; 32]), None);
        let mapped = map.into_option();
        assert!(
            mapped.is_some(),
            "absence markers must be preserved in into_option"
        );
        assert_eq!(mapped.unwrap().len(), 1);
    }

    // ---------------------------------------------------------------------
    // Additional: extend existing malformed request tests to all StateTransitions
    // ---------------------------------------------------------------------

    #[test]
    fn broadcast_state_transition_no_proof_when_response_empty() {
        let request = platform::BroadcastStateTransitionRequest {
            state_transition: vec![],
        };
        let response = platform::WaitForStateTransitionResultResponse::default();
        let provider = unreachable_provider();
        let err = <StateTransitionProofOutcome as FromProof<
            platform::BroadcastStateTransitionRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn broadcast_state_transition_protocol_error_fires_before_metadata_check() {
        // This test pins the ORDERING of validation in
        // `StateTransitionProofOutcome::maybe_from_proof` for broadcast
        // state transitions: proof extraction -> state_transition decode ->
        // metadata check. An invalid state_transition payload triggers
        // `ProtocolError` on decode BEFORE the missing-metadata branch is
        // reached, so even though `metadata: None` here, the assertion
        // targets `ProtocolError`, not `EmptyResponseMetadata`.
        //
        // (For the happy-path `EmptyResponseMetadata` branch, a valid
        // serialized state transition would be needed; that is covered
        // elsewhere. This test deliberately documents the decode-first
        // ordering.)
        use platform::wait_for_state_transition_result_response::{
            wait_for_state_transition_result_response_v0::Result as V0Result, Version,
            WaitForStateTransitionResultResponseV0,
        };
        let request = platform::BroadcastStateTransitionRequest {
            state_transition: vec![0xFFu8; 4],
        };
        let response = platform::WaitForStateTransitionResultResponse {
            version: Some(Version::V0(WaitForStateTransitionResultResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: None, // missing — would trigger EmptyResponseMetadata if reached
            })),
        };
        let provider = unreachable_provider();
        let err = <StateTransitionProofOutcome as FromProof<
            platform::BroadcastStateTransitionRequest,
        >>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            default_platform_version(),
            &provider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::ProtocolError { .. }), "got: {err:?}");
    }
}
