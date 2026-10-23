//! Signs locally produced proofs the way a Tenderdash quorum signs real answers, so tests can
//! run them through the full proof and signature verifier.
//!
//! The quorum key is derived from the fixed secret `[1; 32]`. Recorded vectors made with it are
//! synthetic fixtures, never network trust anchors.
//!
//! The suites using it test drive-proof-verifier but live here, because generating proofs needs
//! drive's server feature, which the client crate must not enable even as a dev-dependency:
//! with resolver 2, `cargo test -p drive-proof-verifier` would compile the library itself
//! against it.
//! Their vectors stay in `rs-drive-proof-verifier/tests/vectors`, next to the crate they verify.

use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use drive_proof_verifier::{ContextProvider, ContextProviderError};
use std::sync::Arc;
use tenderdash_abci::{
    proto::types::{CanonicalVote, SignedMsgType, StateId},
    signatures::{Hashable, Signable},
};

/// A protocol version far above any real one. Recorded vectors signed with it stay newer
/// than every build, so the SDK tests replaying them never depend on the latest version.
pub const NEWER_PROTOCOL_VERSION: u32 = 1_000;

/// Knows no contracts or tokens, and answers the test quorum's public key.
pub struct Provider;
impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        _: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        kind: u32,
        hash: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        assert_eq!((kind, hash), (106, [2; 32]));
        Ok(key().public_key().0.to_compressed())
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}
fn key() -> SecretKey<Bls12381G2Impl> {
    SecretKey::from_be_bytes(&[1; 32]).into_option().unwrap()
}
/// Wraps `bytes` in a proof that the test quorum signed for `root` and `metadata`.
pub fn signed(bytes: Vec<u8>, metadata: &ResponseMetadata, root: [u8; 32]) -> Proof {
    let mut proof = Proof {
        grovedb_proof: bytes,
        quorum_hash: vec![2; 32],
        quorum_type: 106,
        round: 1,
        block_id_hash: vec![3; 32],
        ..Default::default()
    };
    let state = StateId {
        app_version: u64::from(metadata.protocol_version),
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let vote = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: proof.block_id_hash.clone(),
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: i64::from(proof.round),
        state_id: state
            .calculate_msg_hash(
                &metadata.chain_id,
                metadata.height as i64,
                proof.round as i32,
            )
            .unwrap(),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            proof.quorum_type as u8,
            &[2; 32],
            metadata.height as i64,
            proof.round as i32,
        )
        .unwrap();
    proof.signature = key()
        .sign(SignatureSchemes::Basic, &digest)
        .unwrap()
        .as_raw_value()
        .to_compressed()
        .to_vec();
    proof
}
