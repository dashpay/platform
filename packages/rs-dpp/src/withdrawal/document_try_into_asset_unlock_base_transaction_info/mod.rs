use crate::document::Document;
use crate::withdrawal::WithdrawalTransactionIndex;
use crate::ProtocolError;
use dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::AssetUnlockBaseTransactionInfo;
use platform_version::version::PlatformVersion;
mod v0;
impl Document {
    pub fn try_into_asset_unlock_base_transaction_info(
        &self,
        transaction_index: WithdrawalTransactionIndex,
        platform_version: &PlatformVersion,
    ) -> Result<AssetUnlockBaseTransactionInfo, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_method_versions
            .try_into_asset_unlock_base_transaction_info
        {
            0 => self.try_into_asset_unlock_base_transaction_info_v0(transaction_index),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }
}
