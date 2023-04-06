use crate::credits::Credits;
use crate::state_transition::fee::errors::FeeError;
use crate::state_transition::fee::result::ExecutionFees;
use crate::ProtocolError;
use platform_value::Identifier;
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// The balance change for an identity
#[derive(Clone, Debug)]
pub enum BalanceChangeType {
    /// Add Balance
    AddToBalance(Credits),
    /// Remove Balance
    RemoveFromBalance {
        /// the required removed balance
        required_removed_balance: Credits,
        /// the desired removed balance
        desired_removed_balance: Credits,
    },
    /// There was no balance change
    NoBalanceChange,
}

/// The fee expense for the identity from a fee result
#[derive(Clone, Debug)]
pub struct IdentityBalanceChange {
    /// The identifier of the identity
    pub identity_id: Identifier,

    fee_result: ExecutionFees,
    change: BalanceChangeType,
}

impl IdentityBalanceChange {
    pub fn from_fee_result_for_identity(
        fee_result: ExecutionFees,
        identity_id: &Identifier,
    ) -> Self {
        let storage_credits_returned = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity_id)
            .unwrap_or_default();

        let base_required_removed_balance = fee_result.storage_fee;
        let base_desired_removed_balance = fee_result.storage_fee + fee_result.processing_fee;

        let balance_change = match storage_credits_returned.cmp(&base_desired_removed_balance) {
            Ordering::Less => {
                // If we refund more than require to pay we should nil the required
                let required_removed_balance =
                    if storage_credits_returned >= base_required_removed_balance {
                        0
                    } else {
                        // otherwise we should require the difference between them
                        base_required_removed_balance - storage_credits_returned
                    };

                let desired_removed_balance =
                    base_desired_removed_balance - storage_credits_returned;

                BalanceChangeType::RemoveFromBalance {
                    required_removed_balance,
                    desired_removed_balance,
                }
            }
            Ordering::Equal => BalanceChangeType::NoBalanceChange,
            Ordering::Greater => {
                // Credits returned are greater than our spend
                BalanceChangeType::AddToBalance(
                    storage_credits_returned - base_desired_removed_balance,
                )
            }
        };

        Self {
            identity_id: *identity_id,
            fee_result,
            change: balance_change,
        }
    }

    /// Balance change
    pub fn change_type(&self) -> &BalanceChangeType {
        &self.change
    }

    /// Returns refund amount of credits for other identities
    pub fn other_refunds(&self) -> BTreeMap<Identifier, Credits> {
        self.fee_result
            .fee_refunds
            .calculate_all_refunds_except_identity(self.identity_id)
    }

    /// Convert into a fee result
    pub fn into_fee_result(self) -> ExecutionFees {
        self.fee_result
    }

    /// Convert into a fee result minus some processing
    fn into_fee_result_less_processing_debt(self, processing_debt: u64) -> ExecutionFees {
        ExecutionFees {
            processing_fee: self.fee_result.processing_fee - processing_debt,
            ..self.fee_result
        }
    }

    /// The fee result outcome based on user balance
    pub fn fee_result_outcome(self, user_balance: u64) -> Result<ExecutionFees, ProtocolError> {
        match self.change {
            BalanceChangeType::AddToBalance { .. } => {
                // when we add balance we are sure that all the storage fee and processing fee has
                // been payed
                Ok(self.into_fee_result())
            }
            BalanceChangeType::RemoveFromBalance {
                required_removed_balance,
                desired_removed_balance,
            } => {
                if user_balance >= desired_removed_balance {
                    Ok(self.into_fee_result())
                } else if user_balance >= required_removed_balance {
                    // We do not take into account balance debt for total credits balance verification
                    // so we shouldn't add them to pools
                    Ok(self.into_fee_result_less_processing_debt(
                        desired_removed_balance - user_balance,
                    ))
                } else {
                    // The user could not pay for required storage space
                    Err(FeeError::InsufficientBalance.into())
                }
            }
            BalanceChangeType::NoBalanceChange => {
                // while there might be no balance change we still need to deal with refunds
                Ok(self.into_fee_result())
            }
        }
    }
}
