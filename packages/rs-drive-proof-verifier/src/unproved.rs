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

fn parse_hash_32(field: &str, bytes: &[u8]) -> Result<[u8; 32], Error> {
    bytes.try_into().map_err(|_| Error::ProtocolError {
        error: format!(
            "Invalid {field} length: expected 32 bytes, received {}",
            bytes.len()
        ),
    })
}

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
                    .map(|q_hash| parse_hash_32("quorum_hash", &q_hash))
                    .collect::<Result<Vec<[u8; 32]>, Error>>()?;

                // Extract current quorum hash
                let current_quorum_hash =
                    parse_hash_32("current_quorum_hash", &v0.current_quorum_hash)?;

                let last_block_proposer =
                    parse_hash_32("last_block_proposer", &v0.last_block_proposer)?;

                // Extract validator sets
                let validator_sets =
                    v0.validator_sets
                        .into_iter()
                        .map(|vs| {
                            // Parse the ValidatorSetV0
                            let quorum_hash =
                                parse_hash_32("validator_set.quorum_hash", &vs.quorum_hash)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::{
        get_current_quorums_info_response, get_status_response, ResponseMetadata,
    };
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
    use dpp::version::PlatformVersion;

    /// Generate a valid BLS public key as compressed bytes (48 bytes) from a
    /// deterministic secret key derived from the given seed byte.
    fn generate_valid_bls_public_key_bytes(seed: u8) -> Vec<u8> {
        let mut secret_bytes = [0u8; 32];
        secret_bytes[31] = seed.max(1); // ensure nonzero
        let sk: SecretKey<Bls12381G2Impl> =
            SecretKey::<Bls12381G2Impl>::from_be_bytes(&secret_bytes)
                .into_option()
                .expect("valid secret key");
        sk.public_key().0.to_compressed().to_vec()
    }

    /// Helper: build a valid GetCurrentQuorumsInfoResponse with one quorum hash,
    /// one validator set with one member, and metadata.
    fn build_valid_quorums_info_response() -> platform::GetCurrentQuorumsInfoResponse {
        let quorum_hash = vec![1u8; 32];
        let current_quorum_hash = vec![2u8; 32];
        let last_block_proposer = vec![3u8; 32];
        let pro_tx_hash = vec![4u8; 32];
        let threshold_public_key = generate_valid_bls_public_key_bytes(42);

        let member = get_current_quorums_info_response::ValidatorV0 {
            pro_tx_hash: pro_tx_hash.clone(),
            node_ip: "127.0.0.1".to_string(),
            is_banned: false,
        };

        let validator_set = get_current_quorums_info_response::ValidatorSetV0 {
            quorum_hash: quorum_hash.clone(),
            core_height: 100,
            members: vec![member],
            threshold_public_key,
        };

        let v0 = get_current_quorums_info_response::GetCurrentQuorumsInfoResponseV0 {
            quorum_hashes: vec![quorum_hash],
            current_quorum_hash,
            validator_sets: vec![validator_set],
            last_block_proposer,
            metadata: Some(ResponseMetadata {
                height: 500,
                core_chain_locked_height: 200,
                epoch: 10,
                time_ms: 1234567890,
                protocol_version: 1,
                chain_id: "dash-testnet-1".to_string(),
            }),
        };

        platform::GetCurrentQuorumsInfoResponse {
            version: Some(get_current_quorums_info_response::Version::V0(v0)),
        }
    }

    /// Helper: build a valid GetStatusResponse V0 with all inner fields populated.
    fn build_valid_status_response() -> platform::GetStatusResponse {
        use dapi_grpc::platform::v0::get_status_response::get_status_response_v0;

        let software = get_status_response_v0::version::Software {
            dapi: "1.0.0".to_string(),
            drive: Some("2.0.0".to_string()),
            tenderdash: Some("0.14.0".to_string()),
        };

        let tenderdash_protocol =
            get_status_response_v0::version::protocol::Tenderdash { p2p: 8, block: 11 };

        let drive_protocol = get_status_response_v0::version::protocol::Drive {
            latest: 5,
            current: 4,
            next_epoch: 5,
        };

        let protocol = get_status_response_v0::version::Protocol {
            tenderdash: Some(tenderdash_protocol),
            drive: Some(drive_protocol),
        };

        let version = get_status_response_v0::Version {
            software: Some(software),
            protocol: Some(protocol),
        };

        let time = get_status_response_v0::Time {
            local: 1700000000,
            block: Some(1699999900),
            genesis: Some(1690000000),
            epoch: Some(42),
        };

        let node = get_status_response_v0::Node {
            id: vec![10u8; 20],
            pro_tx_hash: Some(vec![11u8; 32]),
        };

        let chain = get_status_response_v0::Chain {
            catching_up: false,
            latest_block_hash: vec![20u8; 32],
            latest_app_hash: vec![21u8; 32],
            latest_block_height: 1000,
            earliest_block_hash: vec![22u8; 32],
            earliest_app_hash: vec![23u8; 32],
            earliest_block_height: 1,
            max_peer_block_height: 1001,
            core_chain_locked_height: Some(500),
        };

        let network = get_status_response_v0::Network {
            chain_id: "dash-testnet-1".to_string(),
            peers_count: 25,
            listening: true,
        };

        let state_sync = get_status_response_v0::StateSync {
            total_synced_time: 3600,
            remaining_time: 120,
            total_snapshots: 5,
            chunk_process_avg_time: 50,
            snapshot_height: 900,
            snapshot_chunks_count: 100,
            backfilled_blocks: 800,
            backfill_blocks_total: 1000,
        };

        let v0 = get_status_response::GetStatusResponseV0 {
            version: Some(version),
            node: Some(node),
            chain: Some(chain),
            network: Some(network),
            state_sync: Some(state_sync),
            time: Some(time),
        };

        platform::GetStatusResponse {
            version: Some(get_status_response::Version::V0(v0)),
        }
    }

    #[test]
    fn test_current_quorums_info_valid_response() {
        let request = platform::GetCurrentQuorumsInfoRequest { version: None };
        let response = build_valid_quorums_info_response();
        let platform_version = PlatformVersion::latest();

        let result = CurrentQuorumsInfo::maybe_from_unproved_with_metadata(
            request,
            response,
            Network::Testnet,
            platform_version,
        );

        let (maybe_info, metadata) = result.expect("should parse valid response");
        let info = maybe_info.expect("should contain CurrentQuorumsInfo");

        assert_eq!(info.quorum_hashes.len(), 1);
        assert_eq!(info.quorum_hashes[0], [1u8; 32]);
        assert_eq!(info.current_quorum_hash, [2u8; 32]);
        assert_eq!(info.last_block_proposer, [3u8; 32]);
        assert_eq!(info.validator_sets.len(), 1);
        assert_eq!(info.last_platform_block_height, 500);
        assert_eq!(info.last_core_block_height, 200);
        assert_eq!(metadata.height, 500);
        assert_eq!(metadata.core_chain_locked_height, 200);
    }

    #[test]
    fn test_current_quorums_info_invalid_quorum_hash_length() {
        let mut response = build_valid_quorums_info_response();

        // Inject an invalid quorum_hash that is not 32 bytes
        if let Some(get_current_quorums_info_response::Version::V0(ref mut v0)) = response.version {
            v0.quorum_hashes = vec![vec![0u8; 16]]; // 16 bytes instead of 32
        }

        let request = platform::GetCurrentQuorumsInfoRequest { version: None };
        let platform_version = PlatformVersion::latest();

        let result = CurrentQuorumsInfo::maybe_from_unproved_with_metadata(
            request,
            response,
            Network::Testnet,
            platform_version,
        );

        let err = result.expect_err("should fail for invalid quorum_hash length");
        let err_string = err.to_string();
        assert!(
            err_string.contains("Invalid quorum_hash length"),
            "unexpected error: {err_string}"
        );
    }

    #[test]
    fn test_current_quorums_info_invalid_current_quorum_hash_length() {
        let mut response = build_valid_quorums_info_response();

        // Inject an invalid current_quorum_hash that is not 32 bytes
        if let Some(get_current_quorums_info_response::Version::V0(ref mut v0)) = response.version {
            v0.current_quorum_hash = vec![0u8; 10]; // 10 bytes instead of 32
        }

        let request = platform::GetCurrentQuorumsInfoRequest { version: None };
        let platform_version = PlatformVersion::latest();

        let result = CurrentQuorumsInfo::maybe_from_unproved_with_metadata(
            request,
            response,
            Network::Testnet,
            platform_version,
        );

        let err = result.expect_err("should fail for invalid current_quorum_hash length");
        let err_string = err.to_string();
        assert!(
            err_string.contains("Invalid current_quorum_hash length"),
            "unexpected error: {err_string}"
        );
    }

    #[test]
    fn test_current_quorums_info_invalid_last_block_proposer() {
        let mut response = build_valid_quorums_info_response();

        // Inject an invalid last_block_proposer that is not 32 bytes
        if let Some(get_current_quorums_info_response::Version::V0(ref mut v0)) = response.version {
            v0.last_block_proposer = vec![0u8; 5]; // 5 bytes instead of 32
        }

        let request = platform::GetCurrentQuorumsInfoRequest { version: None };
        let platform_version = PlatformVersion::latest();

        let result = CurrentQuorumsInfo::maybe_from_unproved_with_metadata(
            request,
            response,
            Network::Testnet,
            platform_version,
        );

        let err = result.expect_err("should fail for invalid last_block_proposer length");
        let err_string = err.to_string();
        assert!(
            err_string.contains("Invalid last_block_proposer length"),
            "unexpected error: {err_string}"
        );
    }

    #[test]
    fn test_current_quorums_info_rejects_malformed_nested_quorum_hashes() {
        for invalid_length in [0, 1, 31, 33, 1024] {
            let mut response = build_valid_quorums_info_response();
            if let Some(get_current_quorums_info_response::Version::V0(ref mut v0)) =
                response.version
            {
                v0.validator_sets[0].quorum_hash = vec![0u8; invalid_length];
            }

            let result = CurrentQuorumsInfo::maybe_from_unproved_with_metadata(
                platform::GetCurrentQuorumsInfoRequest { version: None },
                response,
                Network::Testnet,
                PlatformVersion::latest(),
            );

            let error = result.expect_err("malformed nested quorum hash must return an error");
            assert!(
                matches!(error, Error::ProtocolError { ref error } if error.contains("validator_set.quorum_hash")),
                "unexpected error for length {invalid_length}: {error:?}"
            );
        }
    }

    #[test]
    fn test_current_quorums_info_none_metadata() {
        let mut response = build_valid_quorums_info_response();

        // Remove metadata from the response
        if let Some(get_current_quorums_info_response::Version::V0(ref mut v0)) = response.version {
            v0.metadata = None;
        }

        let request = platform::GetCurrentQuorumsInfoRequest { version: None };
        let platform_version = PlatformVersion::latest();

        let result = CurrentQuorumsInfo::maybe_from_unproved_with_metadata(
            request,
            response,
            Network::Testnet,
            platform_version,
        );

        let err = result.expect_err("should fail when metadata is missing");
        let err_string = err.to_string();
        assert!(
            err_string.contains("empty response metadata"),
            "unexpected error: {err_string}"
        );
    }

    #[test]
    fn test_evo_node_status_valid_response() {
        let request = platform::GetStatusRequest { version: None };
        let response = build_valid_status_response();
        let platform_version = PlatformVersion::latest();

        let result = EvoNodeStatus::maybe_from_unproved_with_metadata(
            request,
            response,
            Network::Testnet,
            platform_version,
        );

        let (maybe_status, _metadata) = result.expect("should parse valid status response");
        let status = maybe_status.expect("should contain EvoNodeStatus");

        // Verify version fields
        let software = status.version.software.as_ref().unwrap();
        assert_eq!(software.dapi, "1.0.0");
        assert_eq!(software.drive.as_deref(), Some("2.0.0"));
        assert_eq!(software.tenderdash.as_deref(), Some("0.14.0"));

        let protocol = status.version.protocol.as_ref().unwrap();
        let td = protocol.tenderdash.as_ref().unwrap();
        assert_eq!(td.p2p, 8);
        assert_eq!(td.block, 11);
        let drv = protocol.drive.as_ref().unwrap();
        assert_eq!(drv.latest, 5);
        assert_eq!(drv.current, 4);
        assert_eq!(drv.next_epoch, 5);

        // Verify node fields
        assert_eq!(status.node.id, vec![10u8; 20]);
        assert_eq!(status.node.pro_tx_hash, Some(vec![11u8; 32]));

        // Verify chain fields
        assert!(!status.chain.catching_up);
        assert_eq!(status.chain.latest_block_height, 1000);
        assert_eq!(status.chain.core_chain_locked_height, Some(500));

        // Verify network fields
        assert_eq!(status.network.chain_id, "dash-testnet-1");
        assert_eq!(status.network.peers_count, 25);
        assert!(status.network.listening);

        // Verify time fields
        assert_eq!(status.time.local, 1700000000);
        assert_eq!(status.time.block, Some(1699999900));
        assert_eq!(status.time.epoch, Some(42));
    }
}
