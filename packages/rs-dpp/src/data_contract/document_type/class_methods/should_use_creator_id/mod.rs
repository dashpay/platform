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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    /// Returns a clone of PlatformVersion::latest() with `should_add_creator_id`
    /// forced to the requested value.
    fn platform_version_with_should_add_creator_id(version: u16) -> PlatformVersion {
        let mut v = PlatformVersion::latest().clone();
        v.dpp
            .contract_versions
            .document_type_versions
            .schema
            .should_add_creator_id = version;
        v
    }

    // ------------------------------------------------------------------
    // Version 0: always returns false regardless of inputs
    // ------------------------------------------------------------------
    #[test]
    fn version_0_always_returns_false_even_if_transferable_and_trade_mode_set() {
        let v = platform_version_with_should_add_creator_id(0);
        let result = should_use_creator_id_class_method(
            1,
            1,
            Transferable::Always,
            TradeMode::DirectPurchase,
            &v,
        )
        .expect("should not error");
        assert!(!result);
    }

    // ------------------------------------------------------------------
    // Version 1: contract_version_type > 0 required
    // ------------------------------------------------------------------
    #[test]
    fn version_1_contract_version_zero_returns_false() {
        let v = platform_version_with_should_add_creator_id(1);
        // contract_version_type = 0 short-circuits to false
        let result = should_use_creator_id_class_method(
            0,
            1,
            Transferable::Always,
            TradeMode::DirectPurchase,
            &v,
        )
        .expect("should not error");
        assert!(!result);
    }

    #[test]
    fn version_1_contract_config_version_zero_returns_false() {
        let v = platform_version_with_should_add_creator_id(1);
        let result = should_use_creator_id_class_method(
            1,
            0,
            Transferable::Always,
            TradeMode::DirectPurchase,
            &v,
        )
        .expect("should not error");
        assert!(!result);
    }

    #[test]
    fn version_1_non_transferable_and_no_trade_mode_returns_false() {
        let v = platform_version_with_should_add_creator_id(1);
        let result =
            should_use_creator_id_class_method(1, 1, Transferable::Never, TradeMode::None, &v)
                .expect("should not error");
        assert!(!result);
    }

    #[test]
    fn version_1_transferable_returns_true() {
        let v = platform_version_with_should_add_creator_id(1);
        let result =
            should_use_creator_id_class_method(1, 1, Transferable::Always, TradeMode::None, &v)
                .expect("should not error");
        assert!(result);
    }

    #[test]
    fn version_1_trade_mode_set_returns_true_even_when_not_transferable() {
        let v = platform_version_with_should_add_creator_id(1);
        let result = should_use_creator_id_class_method(
            1,
            1,
            Transferable::Never,
            TradeMode::DirectPurchase,
            &v,
        )
        .expect("should not error");
        assert!(result);
    }

    #[test]
    fn version_1_both_transferable_and_trade_mode_returns_true() {
        let v = platform_version_with_should_add_creator_id(1);
        let result = should_use_creator_id_class_method(
            1,
            1,
            Transferable::Always,
            TradeMode::DirectPurchase,
            &v,
        )
        .expect("should not error");
        assert!(result);
    }

    // ------------------------------------------------------------------
    // Unknown version error path
    // ------------------------------------------------------------------
    #[test]
    fn unknown_version_returns_unknown_version_mismatch_error() {
        let v = platform_version_with_should_add_creator_id(255);
        let result = should_use_creator_id_class_method(
            1,
            1,
            Transferable::Always,
            TradeMode::DirectPurchase,
            &v,
        );
        assert_matches!(
            result,
            Err(ProtocolError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            }) => {
                assert_eq!(method, "DocumentType::should_use_creator_id");
                assert_eq!(known_versions, vec![0, 1]);
                assert_eq!(received, 255);
            }
        );
    }
}
