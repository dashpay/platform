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

//! Update Genesis Time Operation.
//!

use crate::drive::genesis_time::KEY_GENESIS_TIME;
use crate::drive::RootTree;
use grovedb::batch::GroveDbOp;
use grovedb::Element;

/// Returns a groveDB operation which updates the chain's genesis time with the time given.
pub(super) fn update_genesis_time_operation(genesis_time_ms: u64) -> GroveDbOp {
    // TODO make this into a Op::Replace
    GroveDbOp::insert_run_op(
        vec![vec![RootTree::Pools as u8]],
        KEY_GENESIS_TIME.to_vec(),
        Element::Item(genesis_time_ms.to_be_bytes().to_vec(), None),
    )
}

#[cfg(test)]
mod tests {

    mod update_genesis_time {
        use crate::common::helpers::setup::setup_drive;
        use crate::drive::batch::GroveDbOpBatch;
        use crate::drive::genesis_time::operations::update_genesis_time_operation;
        use crate::error;

        #[test]
        fn test_error_if_fee_pools_is_not_initiated() {
            let drive = setup_drive(None);

            let genesis_time: u64 = 1655396517902;

            let mut batch = GroveDbOpBatch::new();

            batch.push(update_genesis_time_operation(genesis_time));

            match drive.grove_apply_batch(batch, false, None) {
                Ok(_) => assert!(
                    false,
                    "should not be able to update genesis time on uninit fee pools"
                ),
                Err(e) => match e {
                    error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_value_is_set() {
            let drive = setup_drive(None);

            drive
                .create_initial_state_structure(None)
                .expect("expected to create root tree successfully");

            let genesis_time: u64 = 1655396517902;

            let op = update_genesis_time_operation(genesis_time);

            drive
                .grove_apply_operation(op, false, None)
                .expect("should apply batch");

            let stored_genesis_time = drive
                .get_genesis_time(None)
                .expect("should not have an error getting genesis time")
                .expect("should have a genesis time");

            assert_eq!(stored_genesis_time, genesis_time);
        }
    }
}
