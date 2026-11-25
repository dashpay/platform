use crate::error::MapGroveDbError;
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_asset_lock_outpoint_info_request, GetAssetLockOutpointInfoRequest,
    GetAssetLockOutpointInfoResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::dashcore::Network;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetAssetLockOutpointInfoRequest> for StoredAssetLockInfo {
    type Request = GetAssetLockOutpointInfoRequest;
    type Response = GetAssetLockOutpointInfoResponse;

    fn check_request(
        request: &Self::Request,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let get_asset_lock_outpoint_info_request::Version::V0(request) =
            request.version.as_ref().ok_or(Error::EmptyVersion)?;
        if !request.prove {
            return Err(Error::RequestError {
                error: "unproved asset-lock outpoint queries cannot be verified; enable proofs"
                    .to_string(),
            });
        }
        Ok(())
    }

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
        let outpoint = match request.version.ok_or(Error::EmptyVersion)? {
            get_asset_lock_outpoint_info_request::Version::V0(v0) => Bytes36::from_vec(v0.outpoint)
                .map_err(|_| Error::RequestError {
                    error: "outpoint must be exactly 36 bytes".to_string(),
                })?,
        };
        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;
        let (root_hash, info) = Drive::verify_asset_lock_outpoint_info(
            supported_grovedb_proof_bytes(&proof)?,
            &outpoint,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;
        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;
        Ok((Some(info), metadata, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_proof::fixtures::{TestQuorum, QUORUM_HASH, QUORUM_TYPE};
    use dapi_grpc::platform::v0::get_asset_lock_outpoint_info_response::{
        get_asset_lock_outpoint_info_response_v0::Result as ResponseResult, AssetLockOutpointInfo,
        GetAssetLockOutpointInfoResponseV0, Version,
    };
    use dpp::bls::SecretKey;
    use tenderdash_abci::{
        proto::types::{CanonicalVote, SignedMsgType, StateId},
        signatures::{Hashable, Signable},
    };

    fn request(outpoint: [u8; 36]) -> GetAssetLockOutpointInfoRequest {
        GetAssetLockOutpointInfoRequest {
            version: Some(get_asset_lock_outpoint_info_request::Version::V0(
                get_asset_lock_outpoint_info_request::GetAssetLockOutpointInfoRequestV0 {
                    outpoint: outpoint.to_vec(),
                    prove: true,
                },
            )),
        }
    }

    fn metadata() -> ResponseMetadata {
        ResponseMetadata {
            height: 100,
            core_chain_locked_height: 42,
            epoch: 0,
            time_ms: 1_000_000,
            protocol_version: PlatformVersion::latest().protocol_version,
            chain_id: "asset-lock-test".to_string(),
        }
    }

    // Generated offline with Drive at protocol 16: initialize the state structure,
    // insert empty items at [1; 36] and [2; 36] in SpentAssetLockTransactions, then
    // call prove_asset_lock_outpoint_info([1; 36]). The other stored key is outside
    // the proof's query, so requesting [2; 36] cannot be established by this proof.
    // Keep the golden root independent of the verifier, and sign it as a test quorum.
    fn signed_proof(metadata: &ResponseMetadata) -> Proof {
        let mut proof = Proof {
            grovedb_proof: hex::decode(
                include_str!("../../tests/vectors/asset-lock-outpoint/proof.hex").trim(),
            )
            .expect("fixture proof hex"),
            quorum_hash: QUORUM_HASH.to_vec(),
            signature: vec![],
            round: 1,
            block_id_hash: vec![3; 32],
            quorum_type: QUORUM_TYPE,
        };
        let root =
            hex::decode(include_str!("../../tests/vectors/asset-lock-outpoint/root.hex").trim())
                .expect("fixture root hex");
        let state_id = StateId {
            app_version: metadata.protocol_version as u64,
            core_chain_locked_height: metadata.core_chain_locked_height,
            time: metadata.time_ms,
            app_hash: root,
            height: metadata.height,
        };
        let vote = CanonicalVote {
            r#type: SignedMsgType::Precommit.into(),
            block_id: proof.block_id_hash.clone(),
            chain_id: metadata.chain_id.clone(),
            height: metadata.height as i64,
            round: proof.round as i64,
            state_id: state_id
                .calculate_msg_hash(
                    &metadata.chain_id,
                    metadata.height as i64,
                    proof.round as i32,
                )
                .expect("hash signed state"),
        };
        let digest = vote
            .calculate_sign_hash(
                &metadata.chain_id,
                QUORUM_TYPE as u8,
                &QUORUM_HASH,
                metadata.height as i64,
                proof.round as i32,
            )
            .expect("hash vote");
        // TestQuorum derives the key from QUORUM_HASH's first byte plus one.
        let key = SecretKey::from_be_bytes(&[3; 32]).expect("test scalar");
        proof.signature = key
            .sign(&digest)
            .expect("sign fixture root")
            .to_bytes()
            .to_vec();
        proof
    }

    fn response(
        metadata: Option<ResponseMetadata>,
        result: Option<ResponseResult>,
    ) -> GetAssetLockOutpointInfoResponse {
        GetAssetLockOutpointInfoResponse {
            version: Some(Version::V0(GetAssetLockOutpointInfoResponseV0 {
                metadata,
                result,
            })),
        }
    }

    #[allow(clippy::result_large_err)]
    fn verify(
        request: GetAssetLockOutpointInfoRequest,
        response: GetAssetLockOutpointInfoResponse,
        provider: &TestQuorum,
    ) -> Result<(Option<StoredAssetLockInfo>, ResponseMetadata, Proof), Error> {
        StoredAssetLockInfo::maybe_from_proof_with_metadata(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            provider,
        )
    }

    #[test]
    fn should_reject_unproved_requests_before_sending() {
        let mut request = request([1; 36]);
        let Some(get_asset_lock_outpoint_info_request::Version::V0(v0)) = &mut request.version
        else {
            panic!("v0 request");
        };
        v0.prove = false;
        let error =
            <StoredAssetLockInfo as FromProof<GetAssetLockOutpointInfoRequest>>::check_request(
                &request,
                PlatformVersion::latest(),
            )
            .expect_err("unproved requests cannot be verified");
        assert!(matches!(error, Error::RequestError { .. }));
    }

    #[test]
    fn should_accept_quorum_signed_asset_lock_info() {
        let metadata = metadata();
        let proof = signed_proof(&metadata);
        let provider = TestQuorum::new();
        let (info, returned_metadata, returned_proof) = verify(
            request([1; 36]),
            response(
                Some(metadata.clone()),
                Some(ResponseResult::Proof(proof.clone())),
            ),
            &provider,
        )
        .expect("quorum-signed outpoint state");
        assert_eq!(info, Some(StoredAssetLockInfo::FullyConsumed));
        assert_eq!(returned_metadata, metadata);
        assert_eq!(returned_proof, proof);
        assert_eq!(
            *provider.verified.lock().expect("verified heights"),
            vec![(QUORUM_TYPE, QUORUM_HASH, metadata.core_chain_locked_height)]
        );
    }

    fn expect_signature_rejection(metadata: ResponseMetadata, proof: Proof, provider: &TestQuorum) {
        let error = verify(
            request([1; 36]),
            response(Some(metadata), Some(ResponseResult::Proof(proof))),
            provider,
        )
        .expect_err("unauthenticated state must not be returned");
        assert!(
            matches!(
                error,
                Error::InvalidSignature { .. } | Error::SignatureVerificationError { .. }
            ),
            "unexpected error: {error:?}"
        );
        assert!(provider
            .verified
            .lock()
            .expect("verified heights")
            .is_empty());
    }

    #[test]
    fn should_reject_altered_quorum_signature() {
        let metadata = metadata();
        let mut proof = signed_proof(&metadata);
        proof.signature[0] ^= 1;
        expect_signature_rejection(metadata, proof, &TestQuorum::new());
    }

    #[test]
    fn should_reject_wrong_quorum_key() {
        let metadata = metadata();
        let proof = signed_proof(&metadata);
        expect_signature_rejection(metadata, proof, &TestQuorum::with_another_key());
    }

    #[test]
    fn should_reject_altered_signed_metadata() {
        let original = metadata();
        let proof = signed_proof(&original);
        let mut variants = [
            original.clone(),
            original.clone(),
            original.clone(),
            original,
        ];
        variants[0].height += 1;
        variants[1].time_ms += 1;
        variants[2].core_chain_locked_height += 1;
        variants[3].protocol_version += 1;
        for metadata in variants {
            expect_signature_rejection(metadata, proof.clone(), &TestQuorum::new());
        }
    }

    #[test]
    fn should_reject_missing_response_metadata() {
        let provider = TestQuorum::new();
        let error = verify(
            request([1; 36]),
            response(None, Some(ResponseResult::Proof(signed_proof(&metadata())))),
            &provider,
        )
        .expect_err("metadata required");
        assert!(matches!(error, Error::EmptyResponseMetadata));
        assert!(provider
            .verified
            .lock()
            .expect("verified heights")
            .is_empty());
    }

    #[test]
    fn should_reject_unproved_or_missing_result() {
        for result in [
            Some(ResponseResult::AssetLockOutpointInfo(
                AssetLockOutpointInfo {
                    status: 3,
                    remaining_credits: None,
                },
            )),
            None,
        ] {
            let provider = TestQuorum::new();
            let error = verify(
                request([1; 36]),
                response(Some(metadata()), result),
                &provider,
            )
            .expect_err("single-node status is not a proof");
            assert!(matches!(error, Error::NoProofInResult));
            assert!(provider
                .verified
                .lock()
                .expect("verified heights")
                .is_empty());
        }
    }

    #[test]
    fn should_reject_proof_that_cannot_establish_requested_outpoint() {
        let metadata = metadata();
        let provider = TestQuorum::new();
        let error = verify(
            request([2; 36]),
            response(
                Some(metadata.clone()),
                Some(ResponseResult::Proof(signed_proof(&metadata))),
            ),
            &provider,
        )
        .expect_err("proof for another outpoint");
        assert!(
            matches!(error, Error::GroveDBError { .. }),
            "unexpected error: {error:?}"
        );
        assert!(provider
            .verified
            .lock()
            .expect("verified heights")
            .is_empty());
    }
}
