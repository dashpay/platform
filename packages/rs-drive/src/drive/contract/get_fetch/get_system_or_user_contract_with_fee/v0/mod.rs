use crate::drive::contract::get_fetch::get_system_or_user_contract_with_fee::ContractFetchOutcome;
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
    /// at `epoch` (billed — including when the contract is absent, since the lookup still ran).
    pub(super) fn get_system_or_user_contract_with_fee_v0(
        &self,
        contract_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractFetchOutcome, Error> {
        if let Some(system_contract) = self.cache.system_data_contracts.find_by_id(
            Identifier::new(contract_id),
            platform_version.protocol_version,
        ) {
            return Ok(ContractFetchOutcome::System(system_contract));
        }

        let (fee, fetch_info) = self.get_contract_with_fetch_info_and_fee(
            contract_id,
            Some(epoch),
            false,
            transaction,
            platform_version,
        )?;
        // `get_contract_with_fetch_info_and_fee` populates `fee_result` from the
        // accumulated drive operations whenever an epoch is supplied — so `fee`
        // is `Some(...)` for both the found and not-found cases. A `None` fee
        // would indicate an internal invariant violation.
        let Some(fee) = fee else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "get_contract_with_fetch_info_and_fee returned no fee despite being \
                 called with Some(epoch)",
            )));
        };
        match fetch_info {
            Some(fetch_info) => Ok(ContractFetchOutcome::User { fetch_info, fee }),
            None => Ok(ContractFetchOutcome::NotFound { fee }),
        }
    }
}
