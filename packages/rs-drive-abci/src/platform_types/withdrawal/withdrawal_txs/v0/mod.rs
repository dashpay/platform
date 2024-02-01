//! Withdrawal transactions definitions and processing
use crate::abci::AbciError;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::block::block_info::BlockInfo;
use dpp::bls_signatures;
use dpp::fee::fee_result::FeeResult;
use dpp::validation::SimpleValidationResult;
use dpp::version::PlatformVersion;
use drive::{
    drive::{batch::DriveOperation, Drive},
    query::TransactionArg,
};
use std::fmt::Display;
use tenderdash_abci::proto::{
    abci::ExtendVoteExtension,
    types::{VoteExtension, VoteExtensionType},
};
use tenderdash_abci::signatures::Signable;

const MAX_WITHDRAWAL_TXS: u16 = 16;

/// Collection of withdrawal transactions processed at some height/round
#[derive(Debug)]
pub struct WithdrawalTxs<'a> {
    inner: Vec<VoteExtension>,
    drive_operations: Vec<DriveOperation<'a>>,
}

impl<'a> WithdrawalTxs<'a> {
    /// Load pending withdrawal transactions from database
    pub fn load(transaction: TransactionArg, drive: &Drive) -> Result<Self, AbciError> {
        let mut drive_operations = Vec::<DriveOperation>::new();

        let inner = drive
            .dequeue_withdrawal_transactions(MAX_WITHDRAWAL_TXS, transaction, &mut drive_operations)
            .map_err(|e| AbciError::WithdrawalTransactionsDBLoadError(e.to_string()))?
            .into_iter()
            .map(|(_k, v)| VoteExtension {
                r#type: VoteExtensionType::ThresholdRecover.into(),
                extension: v,
                signature: Default::default(),
                sign_request_id: None,
            })
            .collect::<Vec<VoteExtension>>();

        Ok(Self {
            drive_operations,
            inner,
        })
    }

    /// Basic validation of withdrawals.
    ///
    /// TODO: validate signature, etc.
    pub fn validate(&self) -> Result<(), AbciError> {
        if self.drive_operations.len() != self.inner.len() {
            return Err(AbciError::InvalidState(format!(
                "num of drive operations {} must match num of withdrawal transactions {}",
                self.drive_operations.len(),
                self.inner.len(),
            )));
        }

        Ok(())
    }

    /// Finalize operations related to this withdrawal, as part of FinalizeBlock logic.
    ///
    /// Deletes withdrawal transactions that were executed.
    pub fn finalize(
        &self,
        transaction: TransactionArg,
        drive: &Drive,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, AbciError> {
        self.validate()?;
        // TODO: Do we need to do sth with withdrawal txs to actually execute them?
        // FIXME: check if this is correct, esp. "apply" arg
        drive
            .apply_drive_operations(
                self.drive_operations.clone(),
                true,
                block_info,
                transaction,
                platform_version,
            )
            .map_err(|e| AbciError::WithdrawalTransactionsDBLoadError(e.to_string()))
    }
}

impl<'a> WithdrawalTxs<'a> {
    /// Convert withdrawal transactions to vector of ExtendVoteExtension
    pub fn to_vec(&self) -> Vec<ExtendVoteExtension> {
        self.inner
            .iter()
            .map(|v| ExtendVoteExtension {
                r#type: v.r#type,
                extension: v.extension.clone(),
                sign_request_id: None,
            })
            .collect::<Vec<ExtendVoteExtension>>()
    }
    /// Convert withdrawal transactions to vector of ExtendVoteExtension
    pub fn into_vec(self) -> Vec<ExtendVoteExtension> {
        self.inner
            .into_iter()
            .map(|v| ExtendVoteExtension {
                r#type: v.r#type,
                extension: v.extension,
                sign_request_id: None,
            })
            .collect::<Vec<ExtendVoteExtension>>()
    }

    /// Verify signatures of all withdrawal TXs
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
        for s in &self.inner {
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

impl<'a> Display for WithdrawalTxs<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("txs:["))?;
        for item in &self.inner {
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
impl<'a> From<Vec<ExtendVoteExtension>> for WithdrawalTxs<'a> {
    fn from(value: Vec<ExtendVoteExtension>) -> Self {
        WithdrawalTxs {
            inner: value
                .into_iter()
                .map(|v| VoteExtension {
                    r#type: v.r#type,
                    extension: v.extension,
                    signature: Default::default(),
                    sign_request_id: None,
                })
                .collect::<Vec<VoteExtension>>(),
            drive_operations: Vec::<DriveOperation>::new(),
        }
    }
}

impl<'a> From<&Vec<VoteExtension>> for WithdrawalTxs<'a> {
    fn from(value: &Vec<VoteExtension>) -> Self {
        WithdrawalTxs {
            inner: value.clone(),
            drive_operations: Vec::<DriveOperation>::new(),
        }
    }
}

impl<'a> PartialEq for WithdrawalTxs<'a> {
    /// Two sets of withdrawal transactions are equal if all their inner raw transactions are equal.
    ///
    /// ## Notes
    ///
    /// 1. We don't compare `drive_operations`, as this is internal utility fields
    /// 2. For a transaction, we don't compare signatures if at least one of them is empty
    fn eq(&self, other: &Self) -> bool {
        if self.inner.len() != other.inner.len() {
            return false;
        }

        std::iter::zip(&self.inner, &other.inner).all(|(left, right)| {
            left.r#type == right.r#type
                && left.extension == right.extension
                && (left.signature.is_empty()
                    || right.signature.is_empty()
                    || left.signature == right.signature)
        })
    }
}
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

        let mut wt = super::WithdrawalTxs {
            inner: Vec::new(),
            drive_operations: Vec::new(),
        };
        let pubkey = hex::decode("8280cb6694f181db486c59dfa0c6d12d1c4ca26789340aebad0540ffe2edeac387aceec979454c2cfbe75fd8cf04d56d").unwrap();
        let pubkey = bls_signatures::PublicKey::from_bytes(&pubkey).unwrap();

        let signature = hex::decode("A1022D9503CCAFC94FF76FA2E58E10A0474E6EB46305009274FAFCE57E28C7DE57602277777D07855567FAEF6A2F27590258858A875707F4DA32936DDD556BA28455AB04D9301E5F6F0762AC5B9FC036A302EE26116B1F89B74E1457C2D7383A").unwrap();
        // check if signature is correct
        bls_signatures::Signature::from_bytes(&signature).unwrap();
        wt.inner.push(VoteExtension {
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
        wt.inner[0].extension[3] = 0;
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
