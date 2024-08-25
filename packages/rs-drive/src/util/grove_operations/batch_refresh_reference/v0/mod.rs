use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::Element;

impl Drive {
    /// Pushes an "refresh reference" operation to `drive_operations`.
    pub(crate) fn batch_refresh_reference_v0(
        &self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        document_reference: Element,
        trust_refresh_reference: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let Element::Reference(reference_path_type, max_reference_hop, flags) = document_reference
        else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "expected a reference on refresh",
            )));
        };
        drive_operations.push(
            LowLevelDriveOperation::refresh_reference_for_known_path_key_reference_info(
                path,
                key,
                reference_path_type,
                max_reference_hop,
                flags,
                trust_refresh_reference,
            ),
        );
        Ok(())
    }
}
