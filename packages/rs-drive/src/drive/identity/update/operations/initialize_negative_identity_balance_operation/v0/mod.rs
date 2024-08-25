use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::Element;

impl Drive {
    /// Initializes a negative balance operation for an identity.
    ///
    /// This function creates a low-level drive operation to set an identity's balance to zero in GroveDB.
    /// This is typically done when a new identity is created or when the balance needs to be reset.
    ///
    /// # Parameters
    ///
    /// - `identity_id`: A 32-byte array that uniquely identifies the identity whose balance needs to be initialized.
    ///
    /// # Returns
    ///
    /// - `LowLevelDriveOperation`: A low-level drive operation that, when applied, will set the identified identity's
    ///   balance to zero in GroveDB.
    ///
    /// # Usage
    ///
    /// This function is intended to be used internally and should not be exposed to external clients.
    ///
    pub(crate) fn initialize_negative_identity_balance_operation_v0(
        &self,
        identity_id: [u8; 32],
    ) -> LowLevelDriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());

        LowLevelDriveOperation::insert_for_known_path_key_element(
            identity_path,
            vec![IdentityRootStructure::IdentityTreeNegativeCredit as u8],
            Element::new_item(0u64.to_be_bytes().to_vec()),
        )
    }
}
