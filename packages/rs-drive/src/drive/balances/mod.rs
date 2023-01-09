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

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::grove_operations::{DirectQueryType, QueryType};
use crate::drive::system::misc_path;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::operations::insert::InsertOptions;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use integer_encoding::VarInt;

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
pub struct VerifyCreditOutcome {
    /// all the credits in platform
    pub total_credits_in_platform: u64,
    /// all the credits in distribution pools
    pub total_in_pools: u64,
    /// all the credits in identity balances
    pub total_identity_balances: u64,
}

impl VerifyCreditOutcome {
    /// Is the outcome okay? basically do the values match up
    /// Errors in case of overflow
    pub fn ok(&self) -> Result<bool, Error> {
        let VerifyCreditOutcome {
            total_credits_in_platform,
            total_in_pools,
            total_identity_balances,
        } = self;
        let total_from_trees = (*total_in_pools)
            .checked_add(*total_identity_balances)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Overflow of total credits",
            )))?;

        Ok(*total_credits_in_platform == total_from_trees)
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
        self.grove_insert(
            path_holding_total_credits,
            TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
            Item(new_total.encode_var_vec(), None),
            transaction,
            Some(InsertOptions {
                validate_insertion_does_not_override: false,
                validate_insertion_does_not_override_tree: false,
                base_root_storage_is_free: true,
            }),
            &mut drive_operations,
        )
    }

    /// We remove from system credits when:
    /// - an identity withdraws some of their balance
    pub fn remove_from_system_credits(
        &self,
        amount: u64,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
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
        let new_total = total_credits_in_platform
            .checked_sub(amount)
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "trying to remove an amount that would underflow credits",
            )))?;
        self.grove_insert(
            path_holding_total_credits,
            TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
            Item(new_total.encode_var_vec(), None),
            transaction,
            Some(InsertOptions {
                validate_insertion_does_not_override: false,
                validate_insertion_does_not_override_tree: false,
                base_root_storage_is_free: true,
            }),
            &mut drive_operations,
        )
    }

    /// Verify that the sum tree identity credits + pool credits are equal to the
    /// Total credits in the system
    pub fn verify_total_credits(
        &self,
        transaction: TransactionArg,
    ) -> Result<VerifyCreditOutcome, Error> {
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

        if total_identity_balances < 0 {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits of identity balances are less than 0",
            )));
        }
        let total_in_pools = self.grove_get_sum_tree_total_value(
            [],
            Into::<&[u8; 1]>::into(RootTree::Pools),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
        )?;

        if total_in_pools < 0 {
            return Err(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits in distribution pools are less than 0",
            )));
        }

        Ok(VerifyCreditOutcome {
            total_credits_in_platform,
            total_in_pools: total_in_pools as u64,
            total_identity_balances: total_identity_balances as u64,
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
            .verify_total_credits(None)
            .expect("expected to get the result of the verification");
        assert!(credits_match_expected.ok().expect("no overflow"));
    }
}
