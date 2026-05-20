use crate::drive::contract::get_fetch::get_system_or_user_contract_with_fee::FetchedContract;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// v0: short-circuit to the in-memory `SystemDataContracts` cache when the id matches a
    /// system contract (no fee), otherwise delegate to `get_contract_with_fetch_info_and_fee`
    /// at `epoch` (billed).
    pub(super) fn get_system_or_user_contract_with_fee_v0(
        &self,
        contract_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<FetchedContract>, Error> {
        if let Some(system_contract) = self
            .cache
            .system_data_contracts
            .find_by_id(Identifier::new(contract_id))
        {
            return Ok(Some(FetchedContract::System(system_contract)));
        }

        let (fee, fetch_info) = self.get_contract_with_fetch_info_and_fee(
            contract_id,
            Some(epoch),
            false,
            transaction,
            platform_version,
        )?;
        match (fetch_info, fee) {
            (None, _) => Ok(None),
            (Some(fetch_info), Some(fee)) => Ok(Some(FetchedContract::User { fetch_info, fee })),
            // `get_contract_with_fetch_info_and_fee` always returns `Some(fee)` when called with
            // `Some(epoch)`; reaching this arm indicates an internal invariant violation.
            (Some(_), None) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "get_contract_with_fetch_info_and_fee returned a contract without a fee \
                 despite being called with Some(epoch)",
            ))),
        }
    }
}
