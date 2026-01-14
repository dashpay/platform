//! Credits
//!
//! Credits are Platform native token and used for micro payments
//! between identities, state transitions fees and masternode rewards
//!
//! Credits are minted on Platform by locking Dash on payment chain and
//! can be withdrawn back to the payment chain by burning them on Platform
//! and unlocking dash on the payment chain.
//!

use crate::prelude::BlockHeight;
use crate::ProtocolError;
use integer_encoding::VarInt;
use std::collections::BTreeMap;
use std::convert::TryFrom;

/// Duffs type
pub type Duffs = u64;

/// Credits type
pub type Credits = u64;

/// RemainingCredits type
pub type RemainingCredits = Credits;

/// Token Amount type
pub type TokenAmount = u64;

/// Signed Token Amount type
pub type SignedTokenAmount = i64;

/// Sum token amount
pub type SumTokenAmount = i128;

/// Signed Credits type is used for internal computations and total credits
/// balance verification
pub type SignedCredits = i64;

/// Maximum value of credits
pub const MAX_CREDITS: Credits = 9223372036854775807 as Credits; //i64 Max

pub const CREDITS_PER_DUFF: Credits = 1000;

/// An enum for credit operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub enum CreditOperation {
    /// We are setting credit amounts
    SetCredits(Credits),
    /// We are adding to credits
    AddToCredits(Credits),
}

/// An enum for credit operations in compacted address blobs
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub enum BlockAwareCreditOperation {
    /// We are setting credit amounts - the final value after all operations
    SetCredits(Credits),
    /// We are adding to credits - individual additions by block height
    AddToCreditsOperations(BTreeMap<BlockHeight, Credits>),
}

impl BlockAwareCreditOperation {
    /// Merges a CreditOperation from a specific block height into this BlockAwareCreditOperation.
    ///
    /// The merge logic:
    /// - Once a SetCredits is encountered, the result becomes SetCredits with the final computed value
    /// - If only AddToCredits operations, they are preserved with their block heights
    pub fn merge(&mut self, block_height: BlockHeight, operation: &CreditOperation) {
        match (self, operation) {
            // Current is SetCredits, new is SetCredits -> take new value
            (
                BlockAwareCreditOperation::SetCredits(current),
                CreditOperation::SetCredits(new_val),
            ) => {
                *current = *new_val;
            }
            // Current is SetCredits, new is AddToCredits -> add to current value
            (
                BlockAwareCreditOperation::SetCredits(current),
                CreditOperation::AddToCredits(add_val),
            ) => {
                *current = current.saturating_add(*add_val);
            }
            // Current is AddToCredits, new is SetCredits -> compute total of adds before this block + set value
            (
                this @ BlockAwareCreditOperation::AddToCreditsOperations(_),
                CreditOperation::SetCredits(new_val),
            ) => {
                // When we see a SetCredits, all previous AddToCredits don't matter for the final value
                // The SetCredits establishes the baseline
                *this = BlockAwareCreditOperation::SetCredits(*new_val);
            }
            // Current is AddToCredits, new is AddToCredits -> add to map
            (
                BlockAwareCreditOperation::AddToCreditsOperations(map),
                CreditOperation::AddToCredits(add_val),
            ) => {
                map.entry(block_height)
                    .and_modify(|existing| *existing = existing.saturating_add(*add_val))
                    .or_insert(*add_val);
            }
        }
    }

    /// Creates a new BlockAwareCreditOperation from a CreditOperation at a specific block height.
    pub fn from_operation(block_height: BlockHeight, operation: &CreditOperation) -> Self {
        match operation {
            CreditOperation::SetCredits(value) => BlockAwareCreditOperation::SetCredits(*value),
            CreditOperation::AddToCredits(value) => {
                let mut map = BTreeMap::new();
                map.insert(block_height, *value);
                BlockAwareCreditOperation::AddToCreditsOperations(map)
            }
        }
    }
}

impl CreditOperation {
    /// Merges two credit operations, where `other` is applied after `self`.
    ///
    /// The merge logic:
    /// - SetCredits + SetCredits = SetCredits (take the later value)
    /// - SetCredits + AddToCredits = SetCredits (original set value + added amount)
    /// - AddToCredits + SetCredits = SetCredits (take the later value)
    /// - AddToCredits + AddToCredits = AddToCredits (sum of both)
    pub fn merge(&self, other: &CreditOperation) -> CreditOperation {
        match (self, other) {
            // If other is SetCredits, it overrides (it's the most recent set)
            (_, CreditOperation::SetCredits(value)) => CreditOperation::SetCredits(*value),
            // If self is SetCredits and other adds, add to the set value
            (CreditOperation::SetCredits(set_val), CreditOperation::AddToCredits(add_val)) => {
                CreditOperation::SetCredits(set_val.saturating_add(*add_val))
            }
            // If both are AddToCredits, sum them
            (CreditOperation::AddToCredits(val1), CreditOperation::AddToCredits(val2)) => {
                CreditOperation::AddToCredits(val1.saturating_add(*val2))
            }
        }
    }
}

/// Trait for signed and unsigned credits
pub trait Creditable {
    /// Convert unsigned credit to singed
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError>;
    /// Convert singed credit to unsigned
    fn to_unsigned(&self) -> Credits;

    // TODO: Should we implement serialize / unserialize traits instead?

    /// Decode bytes to credits
    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Encode credits to bytes
    fn to_vec_bytes(&self) -> Vec<u8>;
}

impl Creditable for Credits {
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError> {
        SignedCredits::try_from(*self)
            .map_err(|_| ProtocolError::Overflow("credits are too big to convert to signed value"))
    }

    fn to_unsigned(&self) -> Credits {
        *self
    }

    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, ProtocolError> {
        Self::decode_var(vec.as_slice()).map(|(n, _)| n).ok_or(
            ProtocolError::CorruptedSerialization(
                "pending refunds epoch index for must be u16".to_string(),
            ),
        )
    }

    fn to_vec_bytes(&self) -> Vec<u8> {
        self.encode_var_vec()
    }
}

impl Creditable for SignedCredits {
    fn to_signed(&self) -> Result<SignedCredits, ProtocolError> {
        Ok(*self)
    }

    fn to_unsigned(&self) -> Credits {
        self.unsigned_abs()
    }

    fn from_vec_bytes(vec: Vec<u8>) -> Result<Self, ProtocolError> {
        Self::decode_var(vec.as_slice()).map(|(n, _)| n).ok_or(
            ProtocolError::CorruptedSerialization(
                "pending refunds epoch index for must be u16".to_string(),
            ),
        )
    }

    fn to_vec_bytes(&self) -> Vec<u8> {
        self.encode_var_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod block_aware_credit_operation {
        use super::*;

        #[test]
        fn from_operation_set_credits() {
            let op =
                BlockAwareCreditOperation::from_operation(100, &CreditOperation::SetCredits(1000));
            assert_eq!(op, BlockAwareCreditOperation::SetCredits(1000));
        }

        #[test]
        fn from_operation_add_to_credits() {
            let op =
                BlockAwareCreditOperation::from_operation(100, &CreditOperation::AddToCredits(500));
            let expected: BTreeMap<BlockHeight, Credits> = [(100, 500)].into_iter().collect();
            assert_eq!(
                op,
                BlockAwareCreditOperation::AddToCreditsOperations(expected)
            );
        }

        #[test]
        fn merge_set_then_set_takes_latest() {
            let mut op = BlockAwareCreditOperation::SetCredits(1000);
            op.merge(101, &CreditOperation::SetCredits(2000));
            assert_eq!(op, BlockAwareCreditOperation::SetCredits(2000));
        }

        #[test]
        fn merge_set_then_add_adds_to_set() {
            let mut op = BlockAwareCreditOperation::SetCredits(1000);
            op.merge(101, &CreditOperation::AddToCredits(500));
            assert_eq!(op, BlockAwareCreditOperation::SetCredits(1500));
        }

        #[test]
        fn merge_set_then_multiple_adds() {
            let mut op = BlockAwareCreditOperation::SetCredits(1000);
            op.merge(101, &CreditOperation::AddToCredits(500));
            op.merge(102, &CreditOperation::AddToCredits(300));
            assert_eq!(op, BlockAwareCreditOperation::SetCredits(1800));
        }

        #[test]
        fn merge_add_then_set_becomes_set() {
            let mut op =
                BlockAwareCreditOperation::from_operation(100, &CreditOperation::AddToCredits(500));
            op.merge(101, &CreditOperation::SetCredits(2000));
            assert_eq!(op, BlockAwareCreditOperation::SetCredits(2000));
        }

        #[test]
        fn merge_add_then_add_preserves_block_heights() {
            let mut op =
                BlockAwareCreditOperation::from_operation(100, &CreditOperation::AddToCredits(500));
            op.merge(101, &CreditOperation::AddToCredits(300));
            op.merge(102, &CreditOperation::AddToCredits(200));

            let expected: BTreeMap<BlockHeight, Credits> =
                [(100, 500), (101, 300), (102, 200)].into_iter().collect();
            assert_eq!(
                op,
                BlockAwareCreditOperation::AddToCreditsOperations(expected)
            );
        }

        #[test]
        fn merge_multiple_adds_at_same_block_combines() {
            let mut op =
                BlockAwareCreditOperation::from_operation(100, &CreditOperation::AddToCredits(500));
            op.merge(100, &CreditOperation::AddToCredits(300)); // Same block

            let expected: BTreeMap<BlockHeight, Credits> = [(100, 800)].into_iter().collect();
            assert_eq!(
                op,
                BlockAwareCreditOperation::AddToCreditsOperations(expected)
            );
        }

        #[test]
        fn merge_add_then_set_then_add() {
            // AddToCredits(500) at block 100
            let mut op =
                BlockAwareCreditOperation::from_operation(100, &CreditOperation::AddToCredits(500));
            // SetCredits(1000) at block 101 - wipes out the add
            op.merge(101, &CreditOperation::SetCredits(1000));
            // AddToCredits(200) at block 102 - adds to the set
            op.merge(102, &CreditOperation::AddToCredits(200));

            // Result: SetCredits(1200) because Set wiped previous Add, then new Add was applied
            assert_eq!(op, BlockAwareCreditOperation::SetCredits(1200));
        }

        #[test]
        fn client_sync_scenario() {
            // This tests the key use case: client synced at block 550,
            // then receives a compacted range 400-600 with AddToCredits at various blocks.
            // Client should be able to filter and only apply adds for blocks > 550.

            let mut op =
                BlockAwareCreditOperation::from_operation(400, &CreditOperation::AddToCredits(100));
            op.merge(450, &CreditOperation::AddToCredits(200));
            op.merge(500, &CreditOperation::AddToCredits(300));
            op.merge(550, &CreditOperation::AddToCredits(400));
            op.merge(600, &CreditOperation::AddToCredits(500));

            // Verify we have all block heights preserved
            if let BlockAwareCreditOperation::AddToCreditsOperations(map) = &op {
                assert_eq!(map.len(), 5);

                // Client synced at 550, so they need to apply blocks > 550
                let to_apply: Credits = map
                    .iter()
                    .filter(|(block, _)| **block > 550)
                    .map(|(_, credits)| *credits)
                    .sum();

                // Only block 600's AddToCredits(500) should be applied
                assert_eq!(to_apply, 500);

                // Client synced at 400, so they need to apply blocks > 400
                let to_apply_from_400: Credits = map
                    .iter()
                    .filter(|(block, _)| **block > 400)
                    .map(|(_, credits)| *credits)
                    .sum();

                // Blocks 450, 500, 550, 600: 200 + 300 + 400 + 500 = 1400
                assert_eq!(to_apply_from_400, 1400);
            } else {
                panic!("Expected AddToCreditsOperations");
            }
        }

        #[test]
        fn set_credits_followed_by_adds_scenario() {
            // SetCredits at block 400, then adds at 500, 600
            // Client synced at 450, receives range 400-600
            // Client knows balance was SET at 400, so they start from that value
            // and only need to apply adds at blocks > 450

            let mut op =
                BlockAwareCreditOperation::from_operation(400, &CreditOperation::SetCredits(10000));
            op.merge(500, &CreditOperation::AddToCredits(100));
            op.merge(600, &CreditOperation::AddToCredits(200));

            // Result is SetCredits(10300) - all operations merged into final value
            assert_eq!(op, BlockAwareCreditOperation::SetCredits(10300));

            // Note: Once SetCredits is encountered, we lose per-block granularity for adds
            // This is by design - if the balance was SET, client must use the full compacted value
        }
    }
}
