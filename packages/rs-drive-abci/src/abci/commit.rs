//! Tenderdash commit logic
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::bls_signatures::Serialize;
use tenderdash_abci::proto::{self, abci::CommitInfo, signatures::SignDigest, types::BlockId};

use super::AbciError;

/// Represents commit for a block
pub struct Commit {
    inner: proto::types::Commit,
    chain_id: String,
    quorum_type: QuorumType,
}

impl Commit {
    /// Create new Commit struct based on commit info and block id received from Tenderdash
    pub fn new(
        ci: CommitInfo,
        block_id: BlockId,
        height: i64,
        quorum_type: QuorumType,
        chain_id: &str,
    ) -> Self {
        Self {
            chain_id: String::from(chain_id),
            quorum_type: quorum_type,

            inner: proto::types::Commit {
                block_id: Some(block_id),
                height,
                round: ci.round,
                quorum_hash: ci.quorum_hash,
                threshold_block_signature: ci.block_signature,
                threshold_vote_extensions: ci.threshold_vote_extensions,
            },
        }
    }

    /// Verify all signatures using provided public key.
    ///
    /// ## Return value
    ///
    /// * Ok(true) when all signatures are correct
    /// * Ok(false) when at least one signature is invalid
    /// * Err(e) on error
    pub fn verify_signature(
        &self,
        signature: &Vec<u8>,
        public_key: &dpp::bls_signatures::PublicKey,
    ) -> Result<bool, AbciError> {
        // We could have received a fake commit, so signature validation needs to be returned if error as a simple validation result
        let signature = match dpp::bls_signatures::Signature::from_bytes(signature.as_slice()) {
            Ok(signature) => signature,
            Err(e) => return Err(AbciError::from(e)),
        };
        let hash = self
            .inner
            .sign_digest(
                &self.chain_id,
                self.quorum_type as u8,
                self.inner.quorum_hash,
                self.inner.height,
                self.inner.round,
            )
            .map_err(AbciError::TenderdashProto)?;

        Ok(public_key.verify(signature, hash))
    }
}
// impl SignBytes for Commit {
//     fn sign_bytes(&self, chain_id: &str, height: i64, round: i32) -> Result<Vec<u8>, proto::Error> {
//         self.inner.sign_bytes(chain_id, height, round)
//     }
// }

#[cfg(test)]
mod test {
    use super::{request_id, Commit};
    use crate::abci::signature_verifier::Signable;
    use dashcore_rpc::{
        dashcore::hashes::sha256, dashcore::hashes::Hash, dashcore_rpc_json::QuorumType,
    };
    use dpp::bls_signatures::Serialize;
    use tenderdash_abci::proto::{
        abci::CommitInfo,
        signatures::SignBytes,
        types::{BlockId, PartSetHeader, StateId},
    };

    #[test]
    fn test_request_id() {
        let expected =
            hex::decode("28277743e77872951df01bda93a344feca2435e113b8824ce636eada665aadd5")
                .unwrap();
        let got = request_id(super::VOTE_REQUEST_ID_PREFIX, 12, 34);
        assert_eq!(expected, got);
    }
    /*
    vote: Vote{
                    Type:               types.PrecommitType,
                    Height:             1001,
                    ValidatorProTxHash: tmbytes.MustHexDecode("9CC13F685BC3EA0FCA99B87F42ABCC934C6305AA47F62A32266A2B9D55306B7B"),
                },
                quorumHash: tmbytes.MustHexDecode("6A12D9CF7091D69072E254B297AEF15997093E480FDE295E09A7DE73B31CEEDD"),
                want: newSignItem(
                    "C8F2E1FE35DE03AC94F76191F59CAD1BA1F7A3C63742B7125990D996315001CC",
                    "DA25B746781DDF47B5D736F30B1D9D0CC86981EEC67CBE255265C4361DEF8C2E",
                    "02000000E9030000000000000000000000000000E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B"+
                        "7852B855E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855646173682D706C6174666F726D",
                ),
                wantHash: tmbytes.MustHexDecode("0CA3D5F42BDFED0C4FDE7E6DE0F046CC76CDA6CEE734D65E8B2EE0E375D4C57D"),

             */
    #[test]
    fn test_build_sign_hash() {
        let quorum_hash =
            hex::decode("6A12D9CF7091D69072E254B297AEF15997093E480FDE295E09A7DE73B31CEEDD")
                .unwrap();
        let request_id = request_id(super::VOTE_REQUEST_ID_PREFIX, 1001, 0);

        let sign_bytes_hash =
            hex::decode("0CA3D5F42BDFED0C4FDE7E6DE0F046CC76CDA6CEE734D65E8B2EE0E375D4C57D")
                .unwrap();

        let expect_sign_id =
            hex::decode("DA25B746781DDF47B5D736F30B1D9D0CC86981EEC67CBE255265C4361DEF8C2E")
                .unwrap();

        let sign_id = super::build_sign_hash(
            QuorumType::LlmqTest,
            &quorum_hash,
            &request_id,
            &sign_bytes_hash,
        );
        assert_eq!(expect_sign_id, sign_id); // 194,4
    }

    /// Verify that commit signature is correct
    #[test]
    fn test_commit_verify() {
        const HEIGHT: i64 = 12345;
        const ROUND: i32 = 2;
        const CHAIN_ID: &str = "test_chain_id";

        let ci = CommitInfo {
            round: ROUND,
            quorum_hash: vec![0u8; 32],
            ..Default::default()
        };
        let app_hash = [1u8, 2, 3, 4].repeat(8);

        let state_id = StateId {
            height: HEIGHT as u64,
            app_hash,
            app_version: 1,
            core_chain_locked_height: 3,
            time: Some(tenderdash_abci::proto::google::protobuf::Timestamp {
                seconds: 0,
                nanos: 0,
            }),
        };

        let block_id = BlockId {
            hash: sha256::Hash::hash("blockID_hash".as_bytes()).to_vec(),
            part_set_header: Some(PartSetHeader {
                total: 1000000,
                hash: sha256::Hash::hash("blockID_part_set_header_hash".as_bytes()).to_vec(),
            }),
            state_id: state_id
                .sha256(CHAIN_ID, HEIGHT as i64, ROUND as i32)
                .unwrap(),
        };
        let pubkey = hex::decode(
            "b7b76cbef11f48952b4c9778b0cd1e27948c6438c0480e69ce78\
            dc4748611f4463389450a6898f91b08f1de666934324",
        )
        .unwrap();

        let pubkey = dpp::bls_signatures::PublicKey::from_bytes(pubkey.as_slice()).unwrap();
        let signature = hex::decode("95e4a532ccb549cd4feca372b61dd2a5dedea2bb5c33ac22d70e310f\
            7e38126b21029c29e6af6d00462b7c6f5e47047414dbfb2e1008fa0969a246bc38b61e96edddea9c35a01670b0ae45f0\
            8a2626b251bb2a8e937547e65994f2c72d2e8f4e").unwrap();

        let commit = Commit::new(ci, block_id, HEIGHT, QuorumType::LlmqTest, CHAIN_ID);

        let expect_sign_bytes = hex::decode("0200000039300000000000000200000000000000\
            35117edfe49351da1e81d1b0f2edfa0b984a7508958870337126efb352f1210711ae5fef92053e8998c37cb4\
            915968cadfbd2af4fa176b77ade0dadc74028fc5746573745f636861696e5f6964").unwrap();
        let expect_sign_id =
            hex::decode("6f3cb0168cfaf3d9806be8a9eaa85d6ac10e2d32ce02e6a965a66f6c598b06cf")
                .unwrap();
        assert_eq!(
            expect_sign_bytes,
            commit.sign_bytes(CHAIN_ID, HEIGHT, ROUND).unwrap()
        );
        assert_eq!(expect_sign_id, commit.sign_id().unwrap());
        assert!(commit.verify_signature(&signature, &pubkey).unwrap());

        // mutate data and ensure it is invalid
        let mut commit = commit;
        commit.chain_id = "invalid".to_string();
        assert!(!commit.verify_signature(&signature, &pubkey).unwrap());
    }
}
