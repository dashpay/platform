use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity_vec;
use crate::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
#[cfg(feature = "server")]
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fee::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::GroveError;
use crate::query::Query;
#[cfg(feature = "verify")]
use dapi_grpc::platform::v0::{
    get_contested_resource_identity_votes_request::{
        self, get_contested_resource_identity_votes_request_v0,
        GetContestedResourceIdentityVotesRequestV0,
    },
    GetContestedResourceIdentityVotesRequest,
};
use dpp::bincode;
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
#[cfg(feature = "server")]
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::reference_path::ReferencePathType;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{PathQuery, SizedQuery};
#[cfg(feature = "verify")]
use hex::ToHex;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ContestedResourceVotesGivenByIdentityQuery {
    /// Which contestant do we want to get the votes for
    pub identity_id: Identifier,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at vote id
    pub start_at: Option<([u8; 32], bool)>,
    /// Ascending
    pub order_ascending: bool,
}

impl ContestedResourceVotesGivenByIdentityQuery {
    #[cfg(feature = "server")]
    /// Executes a query with proof and returns the items and fee.
    pub fn execute_with_proof(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations = vec![];
        let items = self.execute_with_proof_internal(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with proof and returns the items.
    pub(crate) fn execute_with_proof_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = self.construct_path_query()?;
        drive.grove_get_proved_path_query(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_no_proof_with_cost(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            BTreeMap<Identifier, ContestedDocumentResourceVoteStorageForm>,
            u64,
        ),
        Error,
    > {
        let mut drive_operations = vec![];
        let result =
            self.execute_no_proof(drive, transaction, &mut drive_operations, platform_version)?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((result, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, ContestedDocumentResourceVoteStorageForm>, Error> {
        let path_query = self.construct_path_query()?;
        let query_result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok(BTreeMap::new()),
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => {
                let voters =
                    query_result_elements
                        .to_path_key_elements()
                        .into_iter()
                        .map(|(path, key, element)| {
                            let serialized_reference = element.into_item_bytes()?;
                            let bincode_config = bincode::config::standard()
                                .with_big_endian()
                                .with_no_limit();
                            let reference: ReferencePathType =
                                bincode::decode_from_slice(&serialized_reference, bincode_config)
                                    .map_err(|e| {
                                        Error::Drive(DriveError::CorruptedSerialization(format!(
                                            "serialization of reference {} is corrupted: {}",
                                            hex::encode(serialized_reference),
                                            e
                                        )))
                                    })?
                                    .0;
                            let absolute_path =
                                reference.absolute_path(path.as_slice(), Some(key.as_slice()))?;
                            let vote_id = Identifier::from_vec(key)?;
                            Ok((
                                vote_id,
                                ContestedDocumentResourceVoteStorageForm::try_from_tree_path(
                                    absolute_path,
                                )?,
                            ))
                        })
                        .collect::<Result<
                            BTreeMap<Identifier, ContestedDocumentResourceVoteStorageForm>,
                            Error,
                        >>()?;

                Ok(voters)
            }
        }
    }

    #[cfg(feature = "server")]
    #[allow(unused)]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub(crate) fn execute_no_proof_internal(
        &self,
        drive: &Drive,
        result_type: QueryResultType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(QueryResultElements, u16), Error> {
        let path_query = self.construct_path_query()?;
        let query_result = drive.grove_get_path_query(
            &path_query,
            transaction,
            result_type,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => {
                Ok((QueryResultElements::new(), 0))
            }
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
            }
        }
    }
    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> Result<PathQuery, Error> {
        let path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            self.identity_id.as_bytes(),
        );

        let mut query = Query::new_with_direction(self.order_ascending);

        // this is a range on all elements
        match &self.start_at {
            None => {
                query.insert_all();
            }
            Some((starts_at_key_bytes, start_at_included)) => {
                let starts_at_key = starts_at_key_bytes.to_vec();
                match self.order_ascending {
                    true => match start_at_included {
                        true => query.insert_range_from(starts_at_key..),
                        false => query.insert_range_after(starts_at_key..),
                    },
                    false => match start_at_included {
                        true => query.insert_range_to_inclusive(..=starts_at_key),
                        false => query.insert_range_to(..starts_at_key),
                    },
                }
            }
        }

        Ok(PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: self.limit,
                offset: self.offset,
            },
        })
    }
}

#[cfg(feature = "verify")]
impl TryFrom<GetContestedResourceIdentityVotesRequest>
    for ContestedResourceVotesGivenByIdentityQuery
{
    type Error = Error;
    fn try_from(request: GetContestedResourceIdentityVotesRequest) -> Result<Self, Self::Error> {
        let v0 = match request.version {
            Some(get_contested_resource_identity_votes_request::Version::V0(v)) => v,
            _ => {
                return Err(Error::Protocol(
                    dpp::version::PlatformVersionError::UnknownVersionError(
                        "invalid request version; only version 0 is supported".to_string(),
                    )
                    .into(),
                ))
            }
        };

        Ok(Self {
            identity_id: Identifier::from_vec(v0.identity_id)?,
            offset: v0.offset.map(|x| x as u16),
            limit: v0.limit.map(|x| x as u16),
            start_at: v0
                .start_at_vote_poll_id_info
                .map(|x| {
                    x.start_at_poll_identifier
                        .try_into()
                        .map(|id| (id, x.start_poll_identifier_included))
                })
                .transpose()
                .map_err(|e: <Vec<u8> as TryInto<[u8; 32]>>::Error| {
                    Error::Value(dpp::platform_value::Error::ByteLengthNot32BytesError(
                        format!(
                            "length of start_poll_identifier '0x{}' is not 32 bytes",
                            e.encode_hex::<String>()
                        ),
                    ))
                })?,
            order_ascending: v0.order_ascending,
        })
    }
}

#[cfg(feature = "verify")]
impl From<ContestedResourceVotesGivenByIdentityQuery>
    for GetContestedResourceIdentityVotesRequestV0
{
    fn from(value: ContestedResourceVotesGivenByIdentityQuery) -> Self {
        GetContestedResourceIdentityVotesRequestV0 {
            prove: true, // prove is the default
            identity_id: value.identity_id.to_vec(),
            offset: value.offset.map(|x| x as u32),
            limit: value.limit.map(|x| x as u32),
            start_at_vote_poll_id_info: value.start_at.map(|(id, included)| {
                get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo {
                    start_at_poll_identifier: id.to_vec(),
                    start_poll_identifier_included: included,
                }
            }),
            order_ascending: value.order_ascending,
        }
    }
}
