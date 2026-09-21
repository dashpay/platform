use crate::drive::contract::paths::contract_fee_pots_key;
use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_path;
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    #[inline(always)]
    pub(super) fn insert_contract_fee_pot_trees_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path = prefunded_specialized_balances_path();
        for pot in [ContractFeePot::Owner, ContractFeePot::Moderators] {
            self.grove_insert_if_not_exists(
                (&path).into(),
                contract_fee_pots_key(pot),
                Element::empty_sum_tree(),
                transaction,
                None,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
