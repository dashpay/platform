use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
// If another document type (like V1) ever were to exist we would need to implement max_size_v0 again

impl DocumentTypeV0 {
    pub(in crate::data_contract::document_type) fn max_size_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<u16, ProtocolError> {
        let mut total_size = 0u16;

        for (_, document_property) in self.flattened_properties.iter() {
            let maybe_size = document_property
                .property_type
                .max_byte_size(platform_version)?;

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
