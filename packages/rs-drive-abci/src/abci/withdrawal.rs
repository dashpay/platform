//! Withdrawal transactions definitions and processing

use bls_signatures::{self, BlsError};
use dpp::validation::ValidationResult;
use drive::{
    drive::{batch::DriveOperation, block_info::BlockInfo, Drive},
    fee::result::FeeResult,
    query::TransactionArg,
};
use std::fmt::Display;
use tenderdash_abci::proto::{
    abci::ExtendVoteExtension,
    types::{VoteExtension, VoteExtensionType},
};

use super::{signature_verifier::SignatureVerifier, AbciError};

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
    ) -> Result<FeeResult, AbciError> {
        self.validate()?;
        // TODO: Do we need to do sth with withdrawal txs to actually execute them?
        // FIXME: check if this is correct, esp. "apply" arg
        drive
            .apply_drive_operations(self.drive_operations.clone(), true, block_info, transaction)
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
            })
            .collect::<Vec<ExtendVoteExtension>>()
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
    /// Note we don't compare `drive_operations`.
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<'a> SignatureVerifier for WithdrawalTxs<'a> {
    fn verify_signature(
        &self,
        public_key: &bls_signatures::PublicKey,
    ) -> ValidationResult<bool, BlsError> {
        self.inner.verify_signature(public_key)
    }
}
