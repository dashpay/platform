use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
// If another document type (like V1) ever were to exist we would need to implement estimated_size_v0 again

impl DocumentTypeV0 {
    /// The estimated size uses the middle ceil size of all attributes
    pub(in crate::data_contract::document_type) fn estimated_size_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<u16, ProtocolError> {
        let mut total_size = 0u16;

        for (_, document_property) in self.flattened_properties.iter() {
            // This call now returns a Result<Option<u16>, ProtocolError>.
            let maybe_size = document_property
                .property_type
                .middle_byte_size_ceil(platform_version)?;

            if let Some(size) = maybe_size {
                total_size = match total_size.checked_add(size) {
                    Some(new_total) => new_total,
                    None => {
                        return Ok(u16::MAX);
                    }
                };
            }
        }

        Ok(total_size)
    }
}
