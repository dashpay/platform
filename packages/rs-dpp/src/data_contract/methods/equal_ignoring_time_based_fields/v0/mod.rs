use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::DataContract;

impl DataContract {
    /// Compares two `DataContract` instances while ignoring time-related fields.
    ///
    /// This function checks for equality while excluding:
    /// - `created_at`
    /// - `updated_at`
    /// - `created_at_block_height`
    /// - `updated_at_block_height`
    /// - `created_at_epoch`
    /// - `updated_at_epoch`
    ///
    /// # Arguments
    /// - `other`: A reference to another `DataContract` to compare against.
    ///
    /// # Returns
    /// - `true` if all non-time fields match, otherwise `false`.
    pub(super) fn equal_ignoring_time_fields_v0(&self, other: &DataContract) -> bool {
        self.id() == other.id()
            && self.version() == other.version()
            && self.owner_id() == other.owner_id()
            && self.document_types() == other.document_types()
            && self.config() == other.config()
            && self.groups() == other.groups()
            && self.tokens() == other.tokens()
    }
}
