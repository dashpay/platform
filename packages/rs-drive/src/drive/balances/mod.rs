// MIT LICENSE
//
// Copyright (c) 2023 Dash Core Group
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

//! This module defines functions within the Drive struct related to balances.
//! Functions include inserting verifying balances between various trees.
//!

use crate::drive::grove_operations::DirectQueryType;
use crate::drive::system::{misc_path, misc_path_vec};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::{Creditable, Credits, SignedCredits, MAX_CREDITS};
use crate::fee::op::DriveOperation;
use crate::fee::op::DriveOperation::GroveOperation;
use grovedb::batch::{GroveDbOp, KeyInfoPath};
use grovedb::Element::Item;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;

/// Storage fee pool key
pub const TOTAL_SYSTEM_CREDITS_STORAGE_KEY: &[u8; 1] = b"D";

pub(crate) fn total_credits_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Misc),
        TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
    ]
}

pub(crate) fn balance_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Balances)]
}

pub(crate) fn balance_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Balances).to_vec()]
}

#[derive(Copy, Clone, Debug)]
/// The outcome of verifying credits
pub struct TotalCreditsBalance {
    /// all the credits in platform
    pub total_credits_in_platform: Credits,
    /// all the credits in distribution pools
    pub total_in_pools: SignedCredits,
    /// all the credits in identity balances
    pub total_identity_balances: SignedCredits,
}

impl TotalCreditsBalance {
    /// Is the outcome okay? basically do the values match up
    /// Errors in case of overflow
    pub fn ok(&self) -> Result<bool, Error> {
        let TotalCreditsBalance {
            total_credits_in_platform,
            total_in_pools,
            total_identity_balances,
        } = *self;

        if total_in_pools < 0 {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits in distribution pools are less than 0",
            )));
        }

        if total_identity_balances < 0 {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits of identity balances are less than 0",
            )));
        }

        if total_credits_in_platform > MAX_CREDITS {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Total credits in platform more than max credits size",
            )));
        }

        let total_from_trees = (total_in_pools)
            .checked_add(total_identity_balances)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Overflow of total credits",
            )))?;

        Ok(total_credits_in_platform.to_signed()? == total_from_trees)
    }

    /// Get the total in all trees
    pub fn total_in_trees(&self) -> Result<Credits, Error> {
        let TotalCreditsBalance {
            total_in_pools,
            total_identity_balances,
            ..
        } = *self;

        let total_in_trees =
            total_in_pools
                .checked_add(total_identity_balances)
                .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                    "Overflow of total credits",
                )))?;

        Ok(total_in_trees.to_unsigned())
    }
}

impl Drive {
    /// We add to the total platform system credits when:
    /// - we create an identity
    /// - we top up an identity
    /// - through the block reward
    pub fn add_to_system_credits(
        &self,
        amount: u64,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let mut drive_operations = vec![];
        let batch_operations =
            self.add_to_system_credits_operation(amount, &mut None, transaction)?;
        let grove_db_operations = DriveOperation::grovedb_operations_batch(&batch_operations);
        self.grove_apply_batch_with_add_costs(
            grove_db_operations,
            false,
            transaction,
            &mut drive_operations,
        )
    }

    /// The operations to add to system credits
    pub(crate) fn add_to_system_credits_operation(
        &self,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_total_system_credits_update(
                estimated_costs_only_with_layer_info,
            );
        }
        let path_holding_total_credits = misc_path();
        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                path_holding_total_credits,
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;
        let new_total = total_credits_in_platform
            .checked_add(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to add an amount that would overflow credits",
            )))?;
        let path_holding_total_credits_vec = misc_path_vec();
        let replace_op = GroveDbOp::replace_op(
            path_holding_total_credits_vec,
            TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec(),
            Item(new_total.encode_var_vec(), None),
        );
        drive_operations.push(GroveOperation(replace_op));
        Ok(drive_operations)
    }

    /// We remove from system credits when:
    /// - an identity withdraws some of their balance
    pub fn remove_from_system_credits(
        &self,
        amount: u64,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let mut drive_operations = vec![];
        let batch_operations =
            self.remove_from_system_credits_operations(amount, &mut None, transaction)?;
        let grove_db_operations = DriveOperation::grovedb_operations_batch(&batch_operations);
        self.grove_apply_batch_with_add_costs(
            grove_db_operations,
            false,
            transaction,
            &mut drive_operations,
        )
    }

    /// We remove from system credits when:
    /// - an identity withdraws some of their balance
    pub fn remove_from_system_credits_operations(
        &self,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_total_system_credits_update(
                estimated_costs_only_with_layer_info,
            );
        }
        let path_holding_total_credits = misc_path();
        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                path_holding_total_credits,
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;
        let new_total = total_credits_in_platform
            .checked_sub(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to remove an amount that would underflow credits",
            )))?;
        let path_holding_total_credits_vec = misc_path_vec();
        let replace_op = GroveDbOp::replace_op(
            path_holding_total_credits_vec,
            TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec(),
            Item(new_total.encode_var_vec(), None),
        );
        drive_operations.push(GroveOperation(replace_op));
        Ok(drive_operations)
    }

    /// Verify that the sum tree identity credits + pool credits + refunds are equal to the
    /// Total credits in the system
    pub fn calculate_total_credits_balance(
        &self,
        transaction: TransactionArg,
    ) -> Result<TotalCreditsBalance, Error> {
        let mut drive_operations = vec![];
        let path_holding_total_credits = misc_path();
        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                path_holding_total_credits,
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;

        let total_identity_balances = self.grove_get_sum_tree_total_value(
            [],
            Into::<&[u8; 1]>::into(RootTree::Balances),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
        )?;

        let total_in_pools = self.grove_get_sum_tree_total_value(
            [],
            Into::<&[u8; 1]>::into(RootTree::Pools),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
        )?;

        Ok(TotalCreditsBalance {
            total_credits_in_platform,
            total_in_pools,
            total_identity_balances,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use tempfile::TempDir;

    #[test]
    fn verify_total_credits_structure() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let credits_match_expected = drive
            .calculate_total_credits_balance(None)
            .expect("expected to get the result of the verification");
        assert!(credits_match_expected.ok().expect("no overflow"));
    }
}
