// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Fee Result
//!
//! Each drive operation returns FeeResult after execution.
//! This result contains fees which are required to pay for
//! computation and storage. It also contains fees to refund
//! for removed data from the state.
//!

use crate::consensus::fee::balance_is_not_enough_error::BalanceIsNotEnoughError;
use crate::consensus::fee::fee_error::FeeError;

use crate::fee::fee_result::refunds::FeeRefunds;
use crate::fee::fee_result::BalanceChange::{AddToBalance, NoBalanceChange, RemoveFromBalance};
use crate::fee::Credits;
use crate::prelude::UserFeeIncrease;
use crate::ProtocolError;
use platform_value::Identifier;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::convert::TryFrom;

pub mod refunds;

/// Fee Result
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct FeeResult {
    /// Storage fee
    pub storage_fee: Credits,
    /// Processing fee
    pub processing_fee: Credits,
    /// Credits to refund to identities
    pub fee_refunds: FeeRefunds,
    /// Removed bytes not needing to be refunded to identities
    pub removed_bytes_from_system: u32,
}

impl TryFrom<Vec<FeeResult>> for FeeResult {
    type Error = ProtocolError;
    fn try_from(value: Vec<FeeResult>) -> Result<Self, Self::Error> {
        let mut aggregate_fee_result = FeeResult::default();
        value
            .into_iter()
            .try_for_each(|fee_result| aggregate_fee_result.checked_add_assign(fee_result))?;
        Ok(aggregate_fee_result)
    }
}

impl TryFrom<Vec<Option<FeeResult>>> for FeeResult {
    type Error = ProtocolError;
    fn try_from(value: Vec<Option<FeeResult>>) -> Result<Self, Self::Error> {
        let mut aggregate_fee_result = FeeResult::default();
        value.into_iter().try_for_each(|fee_result| {
            if let Some(fee_result) = fee_result {
                aggregate_fee_result.checked_add_assign(fee_result)
            } else {
                Ok(())
            }
        })?;
        Ok(aggregate_fee_result)
    }
}

/// The balance change for an identity
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BalanceChange {
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
pub struct BalanceChangeForIdentity {
    /// The identifier of the identity
    pub identity_id: Identifier,

    fee_result: FeeResult,
    change: BalanceChange,
}

impl BalanceChangeForIdentity {
    /// Balance change
    pub fn change(&self) -> &BalanceChange {
        &self.change
    }

    /// Returns refund amount of credits for other identities
    pub fn other_refunds(&self) -> BTreeMap<Identifier, Credits> {
        self.fee_result
            .fee_refunds
            .calculate_all_refunds_except_identity(self.identity_id)
    }

    /// Convert into a fee result
    pub fn into_fee_result(self) -> FeeResult {
        self.fee_result
    }

    /// Convert into a fee result minus some processing
    fn into_fee_result_less_processing_debt(self, processing_debt: u64) -> FeeResult {
        FeeResult {
            processing_fee: self.fee_result.processing_fee - processing_debt,
            ..self.fee_result
        }
    }

    /// The fee result outcome based on user balance
    pub fn fee_result_outcome<E>(self, user_balance: u64) -> Result<FeeResult, E>
    where
        E: From<FeeError>,
    {
        match self.change {
            AddToBalance { .. } => {
                // when we add balance we are sure that all the storage fee and processing fee has
                // been payed
                Ok(self.into_fee_result())
            }
            RemoveFromBalance {
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
                    Err(
                        FeeError::BalanceIsNotEnoughError(BalanceIsNotEnoughError::new(
                            user_balance,
                            required_removed_balance,
                        ))
                        .into(),
                    )
                }
            }
            NoBalanceChange => {
                // while there might be no balance change we still need to deal with refunds
                Ok(self.into_fee_result())
            }
        }
    }
}

impl FeeResult {
    /// Convenience method to create a fee result from processing credits
    pub fn new_from_processing_fee(credits: Credits) -> Self {
        Self {
            storage_fee: 0,
            processing_fee: credits,
            fee_refunds: Default::default(),
            removed_bytes_from_system: 0,
        }
    }

    /// Apply a fee multiplier to a fee result
    pub fn apply_user_fee_increase(&mut self, add_fee_percentage_multiplier: UserFeeIncrease) {
        let additional_processing_fee = (self.processing_fee as u128)
            .saturating_mul(add_fee_percentage_multiplier as u128)
            .saturating_div(100);
        if additional_processing_fee > u64::MAX as u128 {
            self.processing_fee = u64::MAX;
        } else {
            self.processing_fee = self
                .processing_fee
                .saturating_add(additional_processing_fee as u64);
        }
    }

    /// Convenience method to get total fee
    pub fn total_base_fee(&self) -> Credits {
        self.storage_fee + self.processing_fee
    }

    /// Convenience method to get required removed balance
    pub fn into_balance_change(self, identity_id: Identifier) -> BalanceChangeForIdentity {
        let storage_credits_returned = self
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity_id)
            .unwrap_or_default();

        let base_required_removed_balance = self.storage_fee;
        let base_desired_removed_balance = self.storage_fee + self.processing_fee;

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

                RemoveFromBalance {
                    required_removed_balance,
                    desired_removed_balance,
                }
            }
            Ordering::Equal => NoBalanceChange,
            Ordering::Greater => {
                // Credits returned are greater than our spend
                AddToBalance(storage_credits_returned - base_desired_removed_balance)
            }
        };

        BalanceChangeForIdentity {
            identity_id,
            fee_result: self,
            change: balance_change,
        }
    }

    /// Creates a FeeResult instance with specified storage and processing fees
    pub fn default_with_fees(storage_fee: Credits, processing_fee: Credits) -> Self {
        FeeResult {
            storage_fee,
            processing_fee,
            ..Default::default()
        }
    }

    /// Adds and self assigns result between two Fee Results
    pub fn checked_add_assign(&mut self, rhs: Self) -> Result<(), ProtocolError> {
        self.storage_fee = self
            .storage_fee
            .checked_add(rhs.storage_fee)
            .ok_or(ProtocolError::Overflow("storage fee overflow error"))?;
        self.processing_fee = self
            .processing_fee
            .checked_add(rhs.processing_fee)
            .ok_or(ProtocolError::Overflow("processing fee overflow error"))?;
        self.fee_refunds.checked_add_assign(rhs.fee_refunds)?;
        self.removed_bytes_from_system = self
            .removed_bytes_from_system
            .checked_add(rhs.removed_bytes_from_system)
            .ok_or(ProtocolError::Overflow(
                "removed_bytes_from_system overflow error",
            ))?;
        Ok(())
    }
}
