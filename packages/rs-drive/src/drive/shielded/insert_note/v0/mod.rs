use crate::drive::shielded::paths::shielded_credit_pool_notes_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use grovedb::batch::QualifiedGroveDbOp;

impl Drive {
    /// Version 0 implementation of constructing a note insertion operation.
    ///
    /// Creates a `commitment_tree_insert_op` that appends the note commitment (cmx)
    /// to the commitment tree frontier and stores `cmx || nullifier || cv_net ||
    /// encrypted_note` as the item value.
    pub(in crate::drive) fn insert_note_op_v0(
        nullifier: [u8; 32],
        cmx: [u8; 32],
        cv_net: [u8; 32],
        encrypted_note: Vec<u8>,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let notes_path = shielded_credit_pool_notes_path_vec();
        Ok(vec![GroveOperation(
            QualifiedGroveDbOp::commitment_tree_insert_op(
                notes_path,
                cmx,
                nullifier,
                cv_net,
                encrypted_note,
            ),
        )])
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;

    #[test]
    fn empty_encrypted_note_still_produces_single_op() {
        // Empty encrypted note payload - edge case - still produces one op.
        let ops = Drive::insert_note_op_v0([0u8; 32], [0u8; 32], [0u8; 32], vec![])
            .expect("should build op even for empty encrypted note");
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn oversized_encrypted_note_still_produces_single_op() {
        // Oversized payload - insert_note_op_v0 itself has no size validation;
        // this verifies the function unconditionally returns a single op.
        let ops = Drive::insert_note_op_v0([1u8; 32], [2u8; 32], [3u8; 32], vec![0xAA; 10_000])
            .expect("oversized payload builds op");
        assert_eq!(ops.len(), 1);
    }
}
