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

//! Genesis Time.
//!
//! This module defines functions relevant to the chain's genesis time.
//!

/// Operations module
pub mod operations;

use crate::drive::genesis_time::operations::update_genesis_time_operation;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::{Element, TransactionArg};
use std::array::TryFromSliceError;

const KEY_GENESIS_TIME: &[u8; 1] = b"g";

impl Drive {
    /// Returns the genesis time. Checks cache first, then storage.
    pub fn get_genesis_time(&self, transaction: TransactionArg) -> Result<Option<u64>, Error> {
        // let's first check the cache
        let mut cache = self.cache.borrow_mut();
        match cache.genesis_time_ms {
            None => {
                let genesis_time_ms = self.fetch_genesis_time(transaction)?;
                if let Some(genesis_time_ms) = genesis_time_ms {
                    // put it into the cache
                    cache.genesis_time_ms = Some(genesis_time_ms);
                }
                Ok(genesis_time_ms)
            }
            Some(genesis_time_ms) => Ok(Some(genesis_time_ms)),
        }
    }

    /// Returns the genesis time from storage.
    fn fetch_genesis_time(&self, transaction: TransactionArg) -> Result<Option<u64>, Error> {
        let element = self
            .grove
            .get(
                [Into::<&[u8; 1]>::into(RootTree::Pools).as_slice()],
                KEY_GENESIS_TIME.as_slice(),
                transaction,
            )
            .unwrap()
            .map(Some)
            .or_else(|e| match e {
                grovedb::Error::PathKeyNotFound(_) => Ok(None),
                _ => Err(e),
            })?;

        if let Some(Element::Item(item, _)) = element {
            let genesis_time = u64::from_be_bytes(item.as_slice().try_into().map_err(
                |e: TryFromSliceError| {
                    Error::Drive(DriveError::CorruptedGenesisTimeInvalidItemLength(
                        e.to_string(),
                    ))
                },
            )?);

            Ok(Some(genesis_time))
        } else {
            Err(Error::Drive(DriveError::CorruptedGenesisTimeNotItem()))
        }
    }

    /// Initializes the genesis time.
    pub fn init_genesis_time(
        &self,
        genesis_time_ms: u64,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.cache.borrow_mut().genesis_time_ms = Some(genesis_time_ms);

        let op = update_genesis_time_operation(genesis_time_ms);

        self.grove_apply_operation(op, false, transaction)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive;
    use crate::drive::genesis_time::KEY_GENESIS_TIME;
    use crate::drive::RootTree;
    use crate::error;
    use grovedb::Element;

    mod fetch_genesis_time {
        use crate::common::helpers::setup::setup_drive;

        #[test]
        fn test_error_if_initial_structure_is_not_initiated() {
            let drive = setup_drive(None);

            match drive.get_genesis_time(None) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = setup_drive(None);

            drive
                .create_initial_state_structure(None)
                .expect("expected to create root tree successfully");

            drive
                .grove
                .insert(
                    [Into::<&[u8; 1]>::into(super::RootTree::Pools).as_slice()],
                    super::KEY_GENESIS_TIME.as_slice(),
                    super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    None,
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_genesis_time(None) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Drive(
                        super::error::drive::DriveError::CorruptedGenesisTimeInvalidItemLength(_),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }

    mod update_genesis_time {
        use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

        #[test]
        fn test_update_genesis_time() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let genesis_time_ms = 100;

            drive
                .init_genesis_time(genesis_time_ms, Some(&transaction))
                .expect("should update genesis time");

            let stored_genesis_time_ms = drive
                .fetch_genesis_time(Some(&transaction))
                .expect("should fetch genesis time");

            match stored_genesis_time_ms {
                Some(stored_genesis_time_ms) => assert_eq!(stored_genesis_time_ms, genesis_time_ms),
                None => assert!(false, "should be present"),
            }

            let cache = drive.cache.borrow();

            match cache.genesis_time_ms {
                Some(stored_genesis_time_ms) => assert_eq!(stored_genesis_time_ms, genesis_time_ms),
                None => assert!(false, "should be present"),
            }
        }
    }
}
