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
    /// to the commitment tree frontier and stores cmx || nullifier || encrypted_note
    /// as the item value.
    pub(in crate::drive) fn insert_note_op_v0(
        nullifier: [u8; 32],
        cmx: [u8; 32],
        encrypted_note: Vec<u8>,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let notes_path = shielded_credit_pool_notes_path_vec();
        Ok(vec![GroveOperation(
            QualifiedGroveDbOp::commitment_tree_insert_op(
                notes_path,
                cmx,
                nullifier,
                encrypted_note,
            ),
        )])
    }
}
