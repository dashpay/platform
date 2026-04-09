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
                // been paid
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
        self.storage_fee.saturating_add(self.processing_fee)
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
                    base_required_removed_balance.saturating_sub(storage_credits_returned);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::fee::fee_error::FeeError;
    use crate::fee::epoch::CreditsPerEpoch;
    use crate::fee::fee_result::refunds::{CreditsPerEpochByIdentifier, FeeRefunds};

    fn make_id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    /// Build a FeeRefunds that gives `credits` to `identity_id` (all in epoch 0).
    fn fee_refunds_for_identity(identity_id: Identifier, credits: Credits) -> FeeRefunds {
        let mut credits_per_epoch = CreditsPerEpoch::default();
        credits_per_epoch.insert(0, credits);
        let mut map = CreditsPerEpochByIdentifier::new();
        map.insert(*identity_id.as_bytes(), credits_per_epoch);
        FeeRefunds(map)
    }

    // --- BalanceChangeForIdentity::change() ---

    #[test]
    fn balance_change_for_identity_change_returns_correct_ref() {
        let id = make_id(1);
        let fee_result = FeeResult::default_with_fees(100, 50);
        let bci = fee_result.into_balance_change(id);
        // No refunds, so it should be RemoveFromBalance
        match bci.change() {
            BalanceChange::RemoveFromBalance {
                required_removed_balance,
                desired_removed_balance,
            } => {
                assert_eq!(*required_removed_balance, 100);
                assert_eq!(*desired_removed_balance, 150);
            }
            other => panic!("Expected RemoveFromBalance, got {:?}", other),
        }
    }

    // --- BalanceChangeForIdentity::other_refunds() ---

    #[test]
    fn other_refunds_empty_when_no_refunds() {
        let id = make_id(1);
        let fee_result = FeeResult::default_with_fees(100, 50);
        let bci = fee_result.into_balance_change(id);
        let refunds = bci.other_refunds();
        assert!(refunds.is_empty());
    }

    #[test]
    fn other_refunds_excludes_own_identity() {
        let id = make_id(1);
        let other_id = make_id(2);
        // Build refunds for both identities
        let mut credits_per_epoch_self = CreditsPerEpoch::default();
        credits_per_epoch_self.insert(0, 200);
        let mut credits_per_epoch_other = CreditsPerEpoch::default();
        credits_per_epoch_other.insert(0, 300);
        let mut map = CreditsPerEpochByIdentifier::new();
        map.insert(*id.as_bytes(), credits_per_epoch_self);
        map.insert(*other_id.as_bytes(), credits_per_epoch_other);
        let refunds = FeeRefunds(map);

        let fee_result = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds,
            removed_bytes_from_system: 0,
        };
        let bci = fee_result.into_balance_change(id);
        let other = bci.other_refunds();
        assert_eq!(other.len(), 1);
        assert_eq!(*other.get(&other_id).unwrap(), 300);
    }

    // --- BalanceChangeForIdentity::into_fee_result() ---

    #[test]
    fn into_fee_result_preserves_original() {
        let fee_result = FeeResult {
            storage_fee: 42,
            processing_fee: 58,
            fee_refunds: FeeRefunds::default(),
            removed_bytes_from_system: 10,
        };
        let id = make_id(1);
        let bci = fee_result.clone().into_balance_change(id);
        let recovered = bci.into_fee_result();
        assert_eq!(recovered.storage_fee, 42);
        assert_eq!(recovered.processing_fee, 58);
        assert_eq!(recovered.removed_bytes_from_system, 10);
    }

    // --- BalanceChangeForIdentity::fee_result_outcome() ---

    #[test]
    fn fee_result_outcome_add_to_balance_returns_fee_result() {
        let id = make_id(1);
        // Refund more than storage + processing so we get AddToBalance
        let refunds = fee_refunds_for_identity(id, 500);
        let fee_result = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds,
            removed_bytes_from_system: 0,
        };
        let bci = fee_result.into_balance_change(id);
        match bci.change() {
            BalanceChange::AddToBalance(amount) => assert_eq!(*amount, 350),
            other => panic!("Expected AddToBalance, got {:?}", other),
        }
        // Cannot access change after move, re-create
        let refunds2 = fee_refunds_for_identity(id, 500);
        let fee_result2 = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds2,
            removed_bytes_from_system: 0,
        };
        let bci2 = fee_result2.into_balance_change(id);
        let result: Result<FeeResult, FeeError> = bci2.fee_result_outcome(0);
        assert!(result.is_ok());
    }

    #[test]
    fn fee_result_outcome_remove_balance_sufficient_desired() {
        let id = make_id(1);
        let fee_result = FeeResult::default_with_fees(100, 50);
        let bci = fee_result.into_balance_change(id);
        // User has enough for desired_removed_balance (150)
        let result: Result<FeeResult, FeeError> = bci.fee_result_outcome(200);
        let fr = result.unwrap();
        assert_eq!(fr.storage_fee, 100);
        assert_eq!(fr.processing_fee, 50);
    }

    #[test]
    fn fee_result_outcome_remove_balance_sufficient_required_but_not_desired() {
        let id = make_id(1);
        let fee_result = FeeResult::default_with_fees(100, 50);
        let bci = fee_result.into_balance_change(id);
        // User has 120: enough for required (100) but not desired (150)
        let result: Result<FeeResult, FeeError> = bci.fee_result_outcome(120);
        let fr = result.unwrap();
        assert_eq!(fr.storage_fee, 100);
        // processing_fee should be reduced by (desired - user_balance) = 150 - 120 = 30
        assert_eq!(fr.processing_fee, 20);
    }

    #[test]
    fn fee_result_outcome_remove_balance_insufficient_returns_error() {
        let id = make_id(1);
        let fee_result = FeeResult::default_with_fees(100, 50);
        let bci = fee_result.into_balance_change(id);
        // User has less than required (100)
        let result: Result<FeeResult, FeeError> = bci.fee_result_outcome(50);
        assert!(result.is_err());
        match result.unwrap_err() {
            FeeError::BalanceIsNotEnoughError(e) => {
                assert_eq!(e.balance(), 50);
                assert_eq!(e.fee(), 100);
            }
        }
    }

    #[test]
    fn fee_result_outcome_no_balance_change_returns_fee_result() {
        let id = make_id(1);
        // Refund exactly storage + processing = 150
        let refunds = fee_refunds_for_identity(id, 150);
        let fee_result = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds,
            removed_bytes_from_system: 0,
        };
        let bci = fee_result.into_balance_change(id);
        match bci.change() {
            BalanceChange::NoBalanceChange => {}
            other => panic!("Expected NoBalanceChange, got {:?}", other),
        }
        // Re-create for outcome check
        let refunds2 = fee_refunds_for_identity(id, 150);
        let fee_result2 = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds2,
            removed_bytes_from_system: 0,
        };
        let bci2 = fee_result2.into_balance_change(id);
        let result: Result<FeeResult, FeeError> = bci2.fee_result_outcome(0);
        assert!(result.is_ok());
    }

    // --- FeeResult::into_balance_change() with 3 ordering branches ---

    #[test]
    fn into_balance_change_less_refund_than_fees() {
        let id = make_id(1);
        // Refund 50, but storage=100 processing=50 total=150
        let refunds = fee_refunds_for_identity(id, 50);
        let fee_result = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds,
            removed_bytes_from_system: 0,
        };
        let bci = fee_result.into_balance_change(id);
        match bci.change() {
            BalanceChange::RemoveFromBalance {
                required_removed_balance,
                desired_removed_balance,
            } => {
                // required = max(0, 100 - 50) = 50
                assert_eq!(*required_removed_balance, 50);
                // desired = 150 - 50 = 100
                assert_eq!(*desired_removed_balance, 100);
            }
            other => panic!("Expected RemoveFromBalance, got {:?}", other),
        }
    }

    #[test]
    fn into_balance_change_refund_equals_fees() {
        let id = make_id(1);
        let refunds = fee_refunds_for_identity(id, 150);
        let fee_result = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds,
            removed_bytes_from_system: 0,
        };
        let bci = fee_result.into_balance_change(id);
        assert_eq!(bci.change(), &BalanceChange::NoBalanceChange);
    }

    #[test]
    fn into_balance_change_refund_greater_than_fees() {
        let id = make_id(1);
        let refunds = fee_refunds_for_identity(id, 300);
        let fee_result = FeeResult {
            storage_fee: 100,
            processing_fee: 50,
            fee_refunds: refunds,
            removed_bytes_from_system: 0,
        };
        let bci = fee_result.into_balance_change(id);
        match bci.change() {
            BalanceChange::AddToBalance(amount) => {
                assert_eq!(*amount, 150); // 300 - 150
            }
            other => panic!("Expected AddToBalance, got {:?}", other),
        }
    }

    #[test]
    fn into_balance_change_no_refunds_no_fees() {
        let id = make_id(1);
        let fee_result = FeeResult::default();
        let bci = fee_result.into_balance_change(id);
        // 0 == 0, so NoBalanceChange? Actually 0.cmp(&0) is Equal
        assert_eq!(bci.change(), &BalanceChange::NoBalanceChange);
    }

    #[test]
    fn into_balance_change_no_refunds_with_fees() {
        let id = make_id(1);
        let fee_result = FeeResult::default_with_fees(200, 100);
        let bci = fee_result.into_balance_change(id);
        match bci.change() {
            BalanceChange::RemoveFromBalance {
                required_removed_balance,
                desired_removed_balance,
            } => {
                assert_eq!(*required_removed_balance, 200);
                assert_eq!(*desired_removed_balance, 300);
            }
            other => panic!("Expected RemoveFromBalance, got {:?}", other),
        }
    }

    // --- apply_user_fee_increase ---

    #[test]
    fn apply_user_fee_increase_zero_percent() {
        let mut fr = FeeResult::default_with_fees(100, 1000);
        fr.apply_user_fee_increase(0);
        assert_eq!(fr.processing_fee, 1000);
    }

    #[test]
    fn apply_user_fee_increase_100_percent() {
        let mut fr = FeeResult::default_with_fees(100, 1000);
        fr.apply_user_fee_increase(100);
        // 100% additional = doubles the processing fee
        assert_eq!(fr.processing_fee, 2000);
    }

    #[test]
    fn apply_user_fee_increase_50_percent() {
        let mut fr = FeeResult::default_with_fees(100, 1000);
        fr.apply_user_fee_increase(50);
        // 50% additional = 1000 + 500
        assert_eq!(fr.processing_fee, 1500);
    }

    #[test]
    fn apply_user_fee_increase_does_not_affect_storage_fee() {
        let mut fr = FeeResult::default_with_fees(500, 1000);
        fr.apply_user_fee_increase(100);
        assert_eq!(fr.storage_fee, 500);
        assert_eq!(fr.processing_fee, 2000);
    }

    #[test]
    fn apply_user_fee_increase_saturates_on_overflow() {
        let mut fr = FeeResult::default_with_fees(0, u64::MAX);
        fr.apply_user_fee_increase(100);
        // Should saturate to u64::MAX rather than panicking
        assert_eq!(fr.processing_fee, u64::MAX);
    }

    #[test]
    fn apply_user_fee_increase_1_percent() {
        let mut fr = FeeResult::default_with_fees(0, 10000);
        fr.apply_user_fee_increase(1);
        // 1% of 10000 = 100
        assert_eq!(fr.processing_fee, 10100);
    }
}
