mod v0;

use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::sync::Arc;

/// Outcome of [`Drive::get_system_or_user_contract_with_fee`].
///
/// A lookup either hits the in-memory system-contract cache (free, no grovedb work),
/// hits storage and finds a user contract (billed), or hits storage and finds nothing
/// (also billed — the grovedb lookup still ran). Callers that need to bill the
/// underlying read should call [`Self::fee`] / [`Self::contract`] or match the variant
/// directly.
#[derive(Debug)]
pub enum ContractFetchOutcome {
    /// Contract was served from the in-memory `SystemDataContracts` cache. No fee.
    System(Arc<DataContract>),
    /// Contract was fetched from grovedb storage. `fee` covers the read.
    User {
        /// The fetched contract and its cached fetch metadata.
        fetch_info: Arc<DataContractFetchInfo>,
        /// The fee charged for the grovedb read that produced `fetch_info`.
        fee: FeeResult,
    },
    /// `contract_id` is not a system contract and storage has no entry for it. The
    /// grovedb lookup that determined this still costs the caller — `fee` covers it.
    NotFound {
        /// The fee charged for the grovedb lookup that returned no contract.
        fee: FeeResult,
    },
}

impl ContractFetchOutcome {
    /// Returns a reference to the underlying `DataContract`, or `None` for `NotFound`.
    pub fn contract(&self) -> Option<&DataContract> {
        match self {
            ContractFetchOutcome::System(arc) => Some(arc.as_ref()),
            ContractFetchOutcome::User { fetch_info, .. } => Some(&fetch_info.contract),
            ContractFetchOutcome::NotFound { .. } => None,
        }
    }

    /// Returns the fee for the lookup that produced this outcome:
    /// - `None` for `System` (served from in-memory cache, no grovedb work).
    /// - `Some(&fee)` for `User` (cost of the read that returned the contract).
    /// - `Some(&fee)` for `NotFound` (cost of the read that proved absence).
    pub fn fee(&self) -> Option<&FeeResult> {
        match self {
            ContractFetchOutcome::System(_) => None,
            ContractFetchOutcome::User { fee, .. } => Some(fee),
            ContractFetchOutcome::NotFound { fee } => Some(fee),
        }
    }
}

impl Drive {
    /// Resolves `contract_id` to one of three outcomes:
    /// - a system contract from the in-memory cache (no fee),
    /// - a user contract fetched from grovedb (billed at `epoch`),
    /// - or absence (still billed — the grovedb lookup ran).
    ///
    /// Selects the implementation via
    /// `platform_version.drive.methods.contract.get.get_system_or_user_contract_with_fee`.
    pub fn get_system_or_user_contract_with_fee(
        &self,
        contract_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractFetchOutcome, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .get
            .get_system_or_user_contract_with_fee
        {
            0 => self.get_system_or_user_contract_with_fee_v0(
                contract_id,
                epoch,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_system_or_user_contract_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
