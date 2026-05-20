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
/// A bound contract is either served from the in-memory system-contract cache (free —
/// no grovedb work) or fetched from storage (billed). Callers that need to bill the
/// underlying read should match `User` and push `fee` into their execution context;
/// `System` is intentionally free.
#[derive(Debug)]
pub enum FetchedContract {
    /// Contract was served from the in-memory `SystemDataContracts` cache. No fee.
    System(Arc<DataContract>),
    /// Contract was fetched from grovedb storage. `fee` covers the read.
    User {
        /// The fetched contract and its cached fetch metadata.
        fetch_info: Arc<DataContractFetchInfo>,
        /// The fee charged for the grovedb read that produced `fetch_info`.
        fee: FeeResult,
    },
}

impl FetchedContract {
    /// Returns a reference to the underlying `DataContract`, regardless of which path
    /// it came from.
    pub fn contract(&self) -> &DataContract {
        match self {
            FetchedContract::System(arc) => arc.as_ref(),
            FetchedContract::User { fetch_info, .. } => &fetch_info.contract,
        }
    }

    /// Returns the read fee for `User`, or `None` for `System` (which is free).
    pub fn fee(&self) -> Option<&FeeResult> {
        match self {
            FetchedContract::System(_) => None,
            FetchedContract::User { fee, .. } => Some(fee),
        }
    }
}

impl Drive {
    /// Resolves `contract_id` either to a system contract from the in-memory cache (free)
    /// or to a user contract fetched from grovedb (billed at `epoch`).
    ///
    /// Returns `Ok(None)` if the id is neither a cached system contract nor present in
    /// storage.
    ///
    /// Selects the implementation via
    /// `platform_version.drive.methods.contract.get.get_system_or_user_contract_with_fee`.
    pub fn get_system_or_user_contract_with_fee(
        &self,
        contract_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<FetchedContract>, Error> {
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
