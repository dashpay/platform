use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{DocumentType, DocumentTypeMutRef, DocumentTypeRef};
use crate::document::transfer::Transferable;
use crate::nft::TradeMode;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DocumentType {
    /// A convenience method on if we should add the creator id
    /// Contracts on version 0 never use creator ids
    pub fn should_use_creator_id(
        &self,
        contract_version_type: u16,
        contract_config_version_type: u16,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        should_use_creator_id_class_method(
            contract_version_type,
            contract_config_version_type,
            self.documents_transferable(),
            self.trade_mode(),
            platform_version,
        )
    }
}

impl DocumentTypeRef<'_> {
    /// A convenience method on if we should add the creator id
    /// Contracts on version 0 never use creator ids
    pub fn should_use_creator_id(
        &self,
        contract_version_type: u16,
        contract_config_version_type: u16,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        should_use_creator_id_class_method(
            contract_version_type,
            contract_config_version_type,
            self.documents_transferable(),
            self.trade_mode(),
            platform_version,
        )
    }
}

impl DocumentTypeMutRef<'_> {
    /// A convenience method on if we should add the creator id
    /// Contracts on version 0 never use creator ids
    pub fn should_use_creator_id(
        &self,
        contract_version_type: u16,
        contract_config_version_type: u16,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        should_use_creator_id_class_method(
            contract_version_type,
            contract_config_version_type,
            self.documents_transferable(),
            self.trade_mode(),
            platform_version,
        )
    }
}

/// A convenience method on if we should add the creator id
fn should_use_creator_id_class_method(
    contract_version_type: u16,
    contract_config_version_type: u16,
    transferable: Transferable,
    trade_mode: TradeMode,
    platform_version: &PlatformVersion,
) -> Result<bool, ProtocolError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .should_add_creator_id
    {
        0 => Ok(false),
        1 => Ok(contract_version_type > 0
            && contract_config_version_type > 0
            && (transferable.is_transferable() || trade_mode != TradeMode::None)),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "DocumentType::should_use_creator_id".to_string(),
            known_versions: vec![0, 1],
            received: version,
        }),
    }
}
