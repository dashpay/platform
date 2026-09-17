use crate::drive::votes::paths::{
    readiness_contract_tree_path, readiness_round_tree_path, READINESS_CURRENT_ROUND_POINTER_KEY,
    READINESS_ROUND_RECORD_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(super) fn fetch_readiness_round_operations_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessRound>, Error> {
        let Some(round_id) = self.fetch_readiness_current_round_id_operations_v0(
            contract_id,
            transaction,
            drive_operations,
            platform_version,
        )?
        else {
            return Ok(None);
        };
        let round = self.fetch_readiness_round_record_operations_v0(
            contract_id,
            round_id,
            transaction,
            drive_operations,
            platform_version,
        )?;
        // The pointer names a round; its record must exist.
        round
            .map(Some)
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "readiness round pointer names a round with no record".to_string(),
            )))
    }

    pub(super) fn fetch_readiness_current_round_id_operations_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        let path = readiness_contract_tree_path(&contract_id);
        let element = match self.grove_get_raw_optional(
            (&path).into(),
            &[READINESS_CURRENT_ROUND_POINTER_KEY as u8],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(element) => element,
            // The contract has no readiness tree at all.
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::InvalidParentLayerPath(_)
                ) =>
            {
                None
            }
            Err(e) => return Err(e),
        };
        match element {
            None => Ok(None),
            Some(Element::Item(bytes, _)) => {
                let round_id: [u8; 32] = bytes.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "readiness round pointer is not 32 bytes".to_string(),
                    ))
                })?;
                Ok(Some(round_id))
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "readiness round pointer was present but was not an item",
            ))),
        }
    }

    pub(super) fn fetch_readiness_round_record_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessRound>, Error> {
        let path = readiness_round_tree_path(&contract_id, &round_id);
        let element = match self.grove_get_raw_optional(
            (&path).into(),
            &[READINESS_ROUND_RECORD_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(element) => element,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::InvalidParentLayerPath(_)
                ) =>
            {
                None
            }
            Err(e) => return Err(e),
        };
        match element {
            None => Ok(None),
            Some(Element::Item(bytes, _)) => {
                Ok(Some(ReadinessRound::deserialize_from_bytes(&bytes)?))
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "readiness round record was present but was not an item",
            ))),
        }
    }
}
