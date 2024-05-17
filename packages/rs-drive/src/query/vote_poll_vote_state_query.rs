use crate::drive::verify::RootHash;
use crate::drive::votes::paths::VotePollPaths;
use crate::drive::votes::resolve_contested_document_resource_vote_poll::{
    ContestedDocumentResourceVotePollResolver, ContestedDocumentResourceVotePollWithContractInfo,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::{GroveError, Query};
use dpp::block::block_info::BlockInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::{PathQuery, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

/// Vote Poll Drive Query result type
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ContestedDocumentVotePollDriveQueryResultType {
    Documents,
    VoteTally,
}

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ContestedDocumentVotePollDriveQuery {
    /// What vote poll are we asking for?
    pub vote_poll: ContestedDocumentResourceVotePoll,
    /// What result type are we interested in
    pub result_type: ContestedDocumentVotePollDriveQueryResultType,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at identity id
    pub start_at: Option<[u8; 32]>,
    /// Start at included
    pub start_at_included: bool,
    /// Ascending
    pub order_ascending: bool,
}

impl ContestedDocumentVotePollDriveQuery {
    pub fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedContestedDocumentVotePollDriveQuery, Error> {
        let ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type,
            offset,
            limit,
            start_at,
            start_at_included,
            order_ascending,
        } = self;
        Ok(ResolvedContestedDocumentVotePollDriveQuery {
            vote_poll: vote_poll.resolve(drive, transaction, platform_version)?,
            result_type: *result_type,
            offset: *offset,
            limit: *limit,
            start_at: *start_at,
            start_at_included: *start_at_included,
            order_ascending: *order_ascending,
        })
    }

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
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
        drive.grove_get_proved_path_query(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    #[cfg(all(feature = "server", feature = "verify"))]
    /// Executes a query with proof and returns the root hash, items, and fee.
    pub fn execute_with_proof_only_get_elements(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>, u64), Error> {
        let mut drive_operations = vec![];
        let (root_hash, items) = self.execute_with_proof_only_get_elements_internal(
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
        Ok((root_hash, items, cost))
    }

    #[cfg(all(feature = "server", feature = "verify"))]
    /// Executes an internal query with proof and returns the root hash and values.
    pub(crate) fn execute_with_proof_only_get_elements_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
        let proof = drive.grove_get_proved_path_query(
            &path_query,
            self.start_at.is_some(),
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        self.verify_proof_keep_serialized(proof.as_slice(), platform_version)
    }

    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_raw_results_no_proof(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations = vec![];
        let (items, skipped) = self.execute_raw_results_no_proof_internal(
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
        Ok((items, skipped, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub(crate) fn execute_raw_results_no_proof_internal(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
        let query_result = drive.grove_get_path_query_serialized_results(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok((Vec::new(), 0)),
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
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
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
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
}

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ResolvedContestedDocumentVotePollDriveQuery {
    /// What vote poll are we asking for?
    pub vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
    /// What result type are we interested in
    pub result_type: ContestedDocumentVotePollDriveQueryResultType,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at identity id
    pub start_at: Option<[u8; 32]>,
    /// Start at included
    pub start_at_included: bool,
    /// Ascending
    pub order_ascending: bool,
}

impl ResolvedContestedDocumentVotePollDriveQuery {
    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let path = self.vote_poll.contenders_path(platform_version)?;

        let mut query = Query::new_with_direction(self.order_ascending);

        // this is a range on all elements
        match &self.start_at {
            None => {
                query.insert_all();
            }
            Some(starts_at_key_bytes) => {
                let starts_at_key = starts_at_key_bytes.to_vec();
                match self.order_ascending {
                    true => match self.start_at_included {
                        true => query.insert_range_from(starts_at_key..),
                        false => query.insert_range_after(starts_at_key..),
                    },
                    false => match self.start_at_included {
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
        let path_query = self.construct_path_query_operations(
            drive,
            true,
            transaction,
            drive_operations,
            platform_version,
        )?;
        drive.grove_get_proved_path_query(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    #[cfg(all(feature = "server", feature = "verify"))]
    /// Executes a query with proof and returns the root hash, items, and fee.
    pub fn execute_with_proof_only_get_elements(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>, u64), Error> {
        let mut drive_operations = vec![];
        let (root_hash, items) = self.execute_with_proof_only_get_elements_internal(
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
        Ok((root_hash, items, cost))
    }

    #[cfg(all(feature = "server", feature = "verify"))]
    /// Executes an internal query with proof and returns the root hash and values.
    pub(crate) fn execute_with_proof_only_get_elements_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let path_query = self.construct_path_query_operations(
            drive,
            true,
            transaction,
            drive_operations,
            platform_version,
        )?;

        let proof = drive.grove_get_proved_path_query(
            &path_query,
            self.start_at.is_some(),
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        self.verify_proof_keep_serialized(proof.as_slice(), platform_version)
    }

    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_raw_results_no_proof(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations = vec![];
        let (items, skipped) = self.execute_raw_results_no_proof_internal(
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
        Ok((items, skipped, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub(crate) fn execute_raw_results_no_proof_internal(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let path_query = self.construct_path_query_operations(
            drive,
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;
        let query_result = drive.grove_get_path_query_serialized_results(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok((Vec::new(), 0)),
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
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
        let path_query = self.construct_path_query_operations(
            drive,
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;
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
}
