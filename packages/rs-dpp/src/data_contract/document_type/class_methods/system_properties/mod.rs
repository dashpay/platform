use crate::data_contract::document_type::DocumentType;
use crate::document::property_names::CREATOR_ID;
use crate::document::transfer::Transferable;
use crate::nft::TradeMode;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

const SYSTEM_PROPERTIES: [&str; 11] = [
    "$id",
    "$ownerId",
    "$createdAt",
    "$updatedAt",
    "$transferredAt",
    "$createdAtBlockHeight",
    "$updatedAtBlockHeight",
    "$transferredAtBlockHeight",
    "$createdAtCoreBlockHeight",
    "$updatedAtCoreBlockHeight",
    "$transferredAtCoreBlockHeight",
];

impl DocumentType {
    pub fn system_properties_contains(
        contract_system_version: u16,
        contract_config_version: u16,
        transferable: Transferable,
        trade_mode: TradeMode,
        property_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .should_add_creator_id
        {
            0 => Ok(SYSTEM_PROPERTIES.contains(&property_name)),
            1 => {
                if property_name == CREATOR_ID
                    && contract_system_version > 0
                    && contract_config_version > 0
                    && (transferable.is_transferable() || trade_mode != TradeMode::None)
                {
                    Ok(true)
                } else {
                    Ok(SYSTEM_PROPERTIES.contains(&property_name))
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentType::system_properties_contains".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
