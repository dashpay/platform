use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::Element;

impl Drive {
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
