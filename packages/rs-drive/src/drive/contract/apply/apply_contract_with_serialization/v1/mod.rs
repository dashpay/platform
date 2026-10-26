use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::DataContract;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

impl Drive {
    /// Generation 1 of the operations applying a contract with its serialization: generation
    /// 0's, but the stored contract is unchanged when its format version 1 bytes are the
    /// applied contract's, not only when its stored bytes are the new serialization. From
    /// protocol version 15 a contract is stored compiled (format version 2), so an unchanged
    /// contract stored before that version is in another format than its new serialization;
    /// applying it writes nothing, and it stays in the format it was stored in.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn apply_contract_with_serialization_operations_v1(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        self.apply_contract_with_serialization_operations_comparing(
            contract,
            contract_serialization,
            block_info,
            estimated_costs_only_with_layer_info,
            storage_flags,
            transaction,
            |stored, serialization| {
                if stored == serialization {
                    return Ok(true);
                }
                let stored_contract =
                    DataContract::versioned_deserialize_trusted(stored, false, platform_version)?;
                Ok(
                    stored_contract.serialize_to_bytes_with_platform_version(platform_version)?
                        == contract.serialize_to_bytes_with_platform_version(platform_version)?,
                )
            },
            platform_version,
        )
    }
}
