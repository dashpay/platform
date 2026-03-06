use crate::types::evonode_status::EvoNodeStatus;
use crate::types::CurrentQuorumsInfo;
use crate::Error;
use dapi_grpc::platform::v0::ResponseMetadata;
use dapi_grpc::platform::v0::{self as platform};
use dapi_grpc::tonic::async_trait;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::core_types::validator::v0::ValidatorV0;
use dpp::core_types::validator_set::v0::ValidatorSetV0;
use dpp::core_types::validator_set::ValidatorSet;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{Network, ProTxHash, PubkeyHash, QuorumHash};
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// Trait for parsing unproved responses from the Platform.
///
/// This trait defines methods for extracting data from responses received from the Platform
/// without the need for cryptographic proof validation. It is primarily used for scenarios where
/// the proof data is not available or not required, and only the data itself is needed.
///
/// ## Associated Types
///
/// - `Request`: The type of the request sent to the server. This represents the format of the
///   data that the platform expects when making a query.
/// - `Response`: The type of the response received from the server. This represents the format of
///   the data returned by the platform after executing the query.
///
/// ## Methods
///
/// - `maybe_from_unproved`: Parses the response to retrieve the requested object, if any.
/// - `maybe_from_unproved_with_metadata`: Parses the response to retrieve the requested object
///   along with response metadata, if any.
/// - `from_unproved`: Retrieves the requested object from the response, returning an error if the
///   object is not found.
/// - `from_unproved_with_metadata`: Retrieves the requested object from the response along with
///   metadata, returning an error if the object is not found.
///
/// ```
pub trait FromUnproved<Req> {
    /// Request type for which this trait is implemented.
    type Request;
    /// Response type for which this trait is implemented.
    type Response;

    /// Parse the received response and retrieve the requested object, if any.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using (Mainnet/Testnet/Devnet/Regtest).
    /// * `platform_version`: The platform version that should be used.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(object))` when the requested object was found in the response.
    /// * `Ok(None)` when the requested object was not found.
    /// * `Err(Error)` when parsing fails or data is invalid.
    fn maybe_from_unproved<I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Self>, Error>
    where
        Self: Sized,
    {
        Self::maybe_from_unproved_with_metadata(request, response, network, platform_version)
            .map(|maybe_result| maybe_result.0)
    }

    /// Parse the received response and retrieve the requested object along with metadata, if any.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using (Mainnet/Testnet/Devnet/Regtest).
    /// * `platform_version`: The platform version that should be used.
    ///
    /// # Returns
    ///
    /// * `Ok((Some(object), metadata))` when the requested object was found.
    /// * `Ok((None, metadata))` when the requested object was not found.
    /// * `Err(Error)` when parsing fails or data is invalid.
    fn maybe_from_unproved_with_metadata<I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Self>, ResponseMetadata), Error>
    where
        Self: Sized;

    /// Retrieve the requested object from the response.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using.
    /// * `platform_version`: The platform version that should be used.
    ///
    /// # Returns
    ///
    /// * `Ok(object)` when the requested object was found.
    /// * `Err(Error::NotFound)` when the requested object was not found.
    /// * `Err(Error)` when parsing fails or data is invalid.
    fn from_unproved<I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::maybe_from_unproved(request, response, network, platform_version)?
            .ok_or(Error::NotFound)
    }

    /// Retrieve the requested object from the response along with metadata.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `network`: The network we are using.
    /// * `platform_version`: The platform version that should be used.
    ///
    /// # Returns
    ///
    /// * `Ok((object, metadata))` when the requested object was found.
    /// * `Err(Error::NotFound)` when the requested object was not found.
    /// * `Err(Error)` when parsing fails or data is invalid.
    fn from_unproved_with_metadata<I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, ResponseMetadata), Error>
    where
        Self: Sized,
    {
        let (main_item, response_metadata) =
            Self::maybe_from_unproved_with_metadata(request, response, network, platform_version)?;
        Ok((main_item.ok_or(Error::NotFound)?, response_metadata))
    }
}

impl FromUnproved<platform::GetCurrentQuorumsInfoRequest> for CurrentQuorumsInfo {
    type Request = platform::GetCurrentQuorumsInfoRequest;
    type Response = platform::GetCurrentQuorumsInfoResponse;

    fn maybe_from_unproved_with_metadata<I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        _platform_version: &PlatformVersion,
    ) -> Result<(Option<Self>, ResponseMetadata), Error>
    where
        Self: Sized,
    {
        // Convert the response into a GetCurrentQuorumsInfoResponse
        let response: platform::GetCurrentQuorumsInfoResponse = response.into();

        // Extract metadata from the response
        let metadata = match &response.version {
            Some(platform::get_current_quorums_info_response::Version::V0(ref v0)) => {
                v0.metadata.clone()
            }
            None => None,
        }
        .ok_or(Error::EmptyResponseMetadata)?;

        // Parse response based on the version field
        let info = match response.version.ok_or(Error::EmptyVersion)? {
            platform::get_current_quorums_info_response::Version::V0(v0) => {
                // Extract quorum hashes
                let quorum_hashes = v0
                    .quorum_hashes
                    .into_iter()
                    .map(|q_hash| {
                        let mut q_hash_array = [0u8; 32];
                        if q_hash.len() != 32 {
                            return Err(Error::ProtocolError {
                                error: "Invalid quorum_hash length".to_string(),
                            });
                        }
                        q_hash_array.copy_from_slice(&q_hash);
                        Ok(q_hash_array)
                    })
                    .collect::<Result<Vec<[u8; 32]>, Error>>()?;

                // Extract current quorum hash
                let mut current_quorum_hash = [0u8; 32];
                if v0.current_quorum_hash.len() != 32 {
                    return Err(Error::ProtocolError {
                        error: "Invalid current_quorum_hash length".to_string(),
                    });
                }
                current_quorum_hash.copy_from_slice(&v0.current_quorum_hash);

                let mut last_block_proposer = [0u8; 32];
                if v0.last_block_proposer.len() != 32 {
                    return Err(Error::ProtocolError {
                        error: "Invalid last_block_proposer length".to_string(),
                    });
                }
                last_block_proposer.copy_from_slice(&v0.last_block_proposer);

                // Extract validator sets
                let validator_sets =
                    v0.validator_sets
                        .into_iter()
                        .map(|vs| {
                            // Parse the ValidatorSetV0
                            let mut quorum_hash = [0u8; 32];
                            quorum_hash.copy_from_slice(&vs.quorum_hash);

                            // Parse ValidatorV0 members
                            let members = vs
                                .members
                                .into_iter()
                                .map(|member| {
                                    let pro_tx_hash = ProTxHash::from_slice(&member.pro_tx_hash)
                                        .map_err(|_| Error::ProtocolError {
                                            error: "Invalid ProTxHash format".to_string(),
                                        })?;
                                    let validator = ValidatorV0 {
                                        pro_tx_hash,
                                        public_key: None, // Assuming it's not provided here
                                        node_ip: member.node_ip,
                                        node_id: PubkeyHash::from_slice(&[0; 20]).expect("expected to make pub key hash from 20 byte empty array"), // Placeholder, since not provided
                                        core_port: 0, // Placeholder, since not provided
                                        platform_http_port: 0, // Placeholder, since not provided
                                        platform_p2p_port: 0, // Placeholder, since not provided
                                        is_banned: member.is_banned,
                                    };
                                    Ok((pro_tx_hash, validator))
                                })
                                .collect::<Result<BTreeMap<ProTxHash, ValidatorV0>, Error>>()?;

                            Ok(ValidatorSet::V0(ValidatorSetV0 {
                                quorum_hash: QuorumHash::from_slice(quorum_hash.as_slice())
                                    .map_err(|_| Error::ProtocolError {
                                        error: "Invalid Quorum Hash format".to_string(),
                                    })?,
                                quorum_index: None, // Assuming it's not provided here
                                core_height: vs.core_height,
                                members,
                                threshold_public_key: BlsPublicKey::try_from(
                                    vs.threshold_public_key.as_slice(),
                                )
                                .map_err(|_| Error::ProtocolError {
                                    error: "Invalid BlsPublicKey format".to_string(),
                                })?,
                            }))
                        })
                        .collect::<Result<Vec<ValidatorSet>, Error>>()?;

                // Create the CurrentQuorumsInfo struct
                Ok::<CurrentQuorumsInfo, Error>(CurrentQuorumsInfo {
                    quorum_hashes,
                    current_quorum_hash,
                    validator_sets,
                    last_block_proposer,
                    last_platform_block_height: metadata.height,
                    last_core_block_height: metadata.core_chain_locked_height,
                })
            }
        }?;

        Ok((Some(info), metadata))
    }
}

#[async_trait]
impl FromUnproved<platform::GetStatusRequest> for EvoNodeStatus {
    type Request = platform::GetStatusRequest;
    type Response = platform::GetStatusResponse;

    fn maybe_from_unproved_with_metadata<I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        response: O,
        _network: Network,
        _platform_version: &PlatformVersion,
    ) -> Result<(Option<Self>, ResponseMetadata), Error>
    where
        Self: Sized,
    {
        let status = Self::try_from(response.into())?;
        // we use default response metadata, as this request does not return any metadata
        Ok((Some(status), Default::default()))
    }
}
