//! Withdrawal transactions definitions and processing
use crate::abci::AbciError;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::bls_signatures;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::{Hash, HashEngine};
use dpp::validation::SimpleValidationResult;
use std::fmt::Display;
use tenderdash_abci::proto::types::VoteExtension;
use tenderdash_abci::signatures::SignDigest;

/// Collection of withdrawal transactions processed at some height/round
#[derive(Debug)]
pub struct SignedWithdrawalTxs(Vec<VoteExtension>);

impl SignedWithdrawalTxs {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> std::slice::Iter<VoteExtension> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Verify signatures of all signed withdrawal TXs
    ///
    /// ## Return value
    ///
    /// There are the following types of errors during verification:
    ///
    /// 1. The signature was invalid, most likely due to change in the data; in this case,
    /// [AbciError::VoteExtensionsSignatureInvalid] is returned.
    /// 2. Signature or public key is malformed - in this case, [AbciError::BlsErrorOfTenderdashThresholdMechanism] is returned
    /// 3. Provided data is invalid - [AbciError::TenderdashProto] is returned
    ///
    /// As all these conditions, in normal circumstances, should cause processing to be terminated, they are all
    /// treated as errors.
    pub fn verify_signatures(
        &self,
        chain_id: &str,
        quorum_type: QuorumType,
        quorum_hash: &[u8],
        height: u64,
        round: u32,
        public_key: &bls_signatures::PublicKey,
    ) -> SimpleValidationResult<AbciError> {
        for s in &self.0 {
            let hash = match s.sign_digest(
                chain_id,
                quorum_type as u8,
                quorum_hash.try_into().expect("invalid quorum hash length"),
                height as i64,
                round as i32,
            ) {
                Ok(h) => h,
                Err(e) => return SimpleValidationResult::new_with_error(AbciError::Tenderdash(e)),
            };

            let signature = match bls_signatures::Signature::from_bytes(&s.signature) {
                Ok(s) => s,
                Err(e) => {
                    return SimpleValidationResult::new_with_error(
                        AbciError::BlsErrorOfTenderdashThresholdMechanism(
                            e,
                            "signature withdrawal verification".to_string(),
                        ),
                    )
                }
            };

            if !public_key.verify(&signature, &hash) {
                return SimpleValidationResult::new_with_error(
                    AbciError::VoteExtensionsSignatureInvalid,
                );
            }
        }

        SimpleValidationResult::default()
    }
}

impl Display for SignedWithdrawalTxs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("txs:["))?;
        for item in &self.0 {
            f.write_fmt(format_args!(
                "tx:{},sig:{}\n",
                hex::encode(&item.extension),
                hex::encode(&item.signature)
            ))?;
        }
        f.write_str("]\n")?;
        Ok(())
    }
}

impl From<&Vec<VoteExtension>> for SignedWithdrawalTxs {
    fn from(value: &Vec<VoteExtension>) -> Self {
        Self(value.clone())
    }
}

// pub fn get_withdrawal_sighash(asset_unlock_tx: &Transaction, quorum_type: u8) -> Vec<u8> {
//     let asset_unlock_payload: AssetUnlockPayload = asset_unlock_tx
//         .clone()
//         .special_transaction_payload
//         .unwrap()
//         .to_asset_unlock_payload()
//         .unwrap();
//
//     let mut request_id_engine = QuorumSigningRequestId::engine();
//     {
//         const ASSET_UNLOCK_REQUEST_ID_PREFIX: &str = "plwdtx";
//         let prefix_len = VarInt(ASSET_UNLOCK_REQUEST_ID_PREFIX.len() as u64);
//         prefix_len.consensus_encode(&mut request_id_engine).unwrap();
//         request_id_engine.input(ASSET_UNLOCK_REQUEST_ID_PREFIX.as_bytes());
//         request_id_engine.input(&asset_unlock_payload.base.index.to_le_bytes());
//     }
//     let request_id = QuorumSigningRequestId::from_engine(request_id_engine);
//
//     let mut signature_hash_engine = QuorumSigningRequestId::engine();
//     {
//         signature_hash_engine.input(&quorum_type.to_le_bytes());
//         signature_hash_engine.input(
//             &asset_unlock_payload
//                 .request_info
//                 .quorum_hash
//                 .as_byte_array()
//                 .as_slice(),
//         );
//         signature_hash_engine.input(&request_id.as_byte_array().as_slice());
//         signature_hash_engine.input(&asset_unlock_tx.txid().as_byte_array().as_slice());
//     }
//     let signature_hash = QuorumSigningRequestId::from_engine(signature_hash_engine);
//
//     Vec::from(signature_hash.as_byte_array().to_owned())
// }
//
// impl<'a> PartialEq for SignedWithdrawalTxs<'a> {
//     /// Two sets of withdrawal transactions are equal if all their inner raw transactions are equal.
//     ///
//     /// ## Notes
//     ///
//     /// 1. We don't compare `drive_operations`, as this is internal utility fields
//     /// 2. For a transaction, we don't compare signatures if at least one of them is empty
//     fn eq(&self, other: &Self) -> bool {
//         if self.inner.len() != other.inner.len() {
//             return false;
//         }
//
//         std::iter::zip(&self.inner, &other.inner).all(|(left, right)| {
//             left.r#type == right.r#type
//                 && left.extension == right.extension
//                 && (left.signature.is_empty()
//                     || right.signature.is_empty()
//                     || left.signature == right.signature)
//         })
//     }
// }
#[cfg(test)]
mod test {
    use dashcore_rpc::dashcore_rpc_json::QuorumType;
    use dpp::bls_signatures;
    use tenderdash_abci::proto::types::{VoteExtension, VoteExtensionType};

    #[test]
    fn verify_signature() {
        const HEIGHT: u64 = 100;
        const ROUND: u32 = 0;
        const CHAIN_ID: &str = "test-chain";

        let quorum_hash =
            hex::decode("D6711FA18C7DA6D3FF8615D3CD3C14500EE91DA5FA942425B8E2B79A30FD8E6C")
                .unwrap();

        let mut wt = super::SignedWithdrawalTxs(Vec::new());
        let pubkey = hex::decode("8280cb6694f181db486c59dfa0c6d12d1c4ca26789340aebad0540ffe2edeac387aceec979454c2cfbe75fd8cf04d56d").unwrap();
        let pubkey = bls_signatures::PublicKey::from_bytes(&pubkey).unwrap();

        let signature = hex::decode("A1022D9503CCAFC94FF76FA2E58E10A0474E6EB46305009274FAFCE57E28C7DE57602277777D07855567FAEF6A2F27590258858A875707F4DA32936DDD556BA28455AB04D9301E5F6F0762AC5B9FC036A302EE26116B1F89B74E1457C2D7383A").unwrap();
        // check if signature is correct
        bls_signatures::Signature::from_bytes(&signature).unwrap();
        wt.0.push(VoteExtension {
            extension: [
                82u8, 79, 29, 3, 209, 216, 30, 148, 160, 153, 4, 39, 54, 212, 11, 217, 104, 27,
                134, 115, 33, 68, 63, 245, 138, 69, 104, 226, 116, 219, 216, 59,
            ]
            .into(),
            signature,
            r#type: VoteExtensionType::ThresholdRecover.into(),
            sign_request_id: None,
        });

        assert!(wt
            .verify_signatures(
                CHAIN_ID,
                QuorumType::LlmqTest,
                &quorum_hash,
                HEIGHT,
                ROUND,
                &pubkey
            )
            .is_valid());

        // Now break the data
        wt.0[0].extension[3] = 0;
        assert!(!wt
            .verify_signatures(
                CHAIN_ID,
                QuorumType::LlmqTest,
                &quorum_hash,
                HEIGHT,
                ROUND,
                &pubkey
            )
            .is_valid());
    }
}
