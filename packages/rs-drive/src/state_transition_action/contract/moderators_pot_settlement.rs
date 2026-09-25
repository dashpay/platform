use crate::error::fee::FeeError;
use crate::error::Error;
use crate::util::batch::DriveOperation::{
    ContractFeePotOperation, ContractModerationOperation, IdentityOperation,
};
use crate::util::batch::{
    ContractFeePotOperationType, ContractModerationOperationType, DriveOperation,
    IdentityOperationType,
};
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use std::collections::BTreeMap;

/// A settle of the moderators pot of a contract with a seated moderation team, as settled when
/// the transition was validated: what each identity of the team is paid by its proposal's
/// reward split, and whose moderation action counts the settle resets. A claim of the pot
/// settles it, and so does every change of the team, beforehand.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModeratorsPotSettlement {
    /// The elected contract whose moderators pot is settled.
    pub contract_id: Identifier,
    /// What each identity is paid: every amount is above zero, and they add up to at most
    /// the pot. May be empty when the pot holds too little to pay anyone a credit.
    pub payouts: BTreeMap<Identifier, Credits>,
    /// The members whose moderation action counts exist and are deleted by the settle.
    pub settled_action_counts: Vec<Identifier>,
}

impl ModeratorsPotSettlement {
    /// Whether the settle changes nothing: nobody is paid and no count is reset.
    pub fn is_empty(&self) -> bool {
        self.payouts.is_empty() && self.settled_action_counts.is_empty()
    }

    /// What is taken out of the pot: the sum of the payouts.
    pub fn paid_out(&self) -> Result<Credits, Error> {
        self.payouts
            .values()
            .try_fold(0 as Credits, |total, amount| total.checked_add(*amount))
            .ok_or(Error::Fee(FeeError::Overflow(
                "the payouts of a moderators pot settle overflow credits",
            )))
    }

    /// The operations of the settle: the credits paid out leave the pot and reach the
    /// balances of the team, so they only move, and the counts the payouts were split by are
    /// deleted.
    pub fn into_drive_operations<'a>(self) -> Result<Vec<DriveOperation<'a>>, Error> {
        let paid_out = self.paid_out()?;
        let ModeratorsPotSettlement {
            contract_id,
            payouts,
            settled_action_counts,
        } = self;
        let mut operations = vec![];
        if paid_out > 0 {
            operations.push(ContractFeePotOperation(
                ContractFeePotOperationType::DeductFromPot {
                    contract_id,
                    pot: ContractFeePot::Moderators,
                    amount: paid_out,
                },
            ));
        }
        operations.extend(payouts.into_iter().map(|(identity_id, added_balance)| {
            IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id: identity_id.to_buffer(),
                added_balance,
            })
        }));
        if !settled_action_counts.is_empty() {
            operations.push(ContractModerationOperation(
                ContractModerationOperationType::RemoveActionCounts {
                    contract_id,
                    identity_ids: settled_action_counts,
                },
            ));
        }
        Ok(operations)
    }
}
