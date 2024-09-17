use crate::drive::contract::paths::DataContractPaths;
use crate::drive::Drive;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::GroveError;
use crate::query::Query;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;
use std::ops::BitXor;

#[cfg(any(feature = "server", feature = "verify"))]
/// Internal clauses struct
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PrimaryContestedInternalClauses {
    /// Primary key in clause
    pub primary_key_in_clause: Option<Vec<Identifier>>,
    /// Primary key equal clause
    pub primary_key_equal_clause: Option<Identifier>,
}

impl PrimaryContestedInternalClauses {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns true if the clause is a valid format.
    pub fn verify(&self) -> bool {
        // There can only be 1 primary key clause
        self.primary_key_in_clause
            .is_some()
            .bitxor(self.primary_key_equal_clause.is_some())
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
/// Drive query struct
#[derive(Debug, PartialEq, Clone)]
pub struct DriveContestedDocumentQuery<'a> {
    ///DataContract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
    /// Internal clauses
    pub internal_clauses: PrimaryContestedInternalClauses,
}

impl<'a> DriveContestedDocumentQuery<'a> {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a path query given a document type path and starting document.
    pub fn construct_path_query(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        // First we should get the overall document_type_path
        let mut path = self
            .contract
            .contested_document_type_path(self.document_type.name().as_str())
            .into_iter()
            .map(|a| a.to_vec())
            .collect::<Vec<Vec<u8>>>();

        // Add primary key ($id) subtree
        path.push(vec![0]);

        if let Some(primary_key_equal_clause) = &self.internal_clauses.primary_key_equal_clause {
            let mut query = Query::new();
            query.insert_key(primary_key_equal_clause.to_vec());

            Ok(PathQuery::new(path, SizedQuery::new(query, Some(1), None)))
        } else {
            let mut query = Query::new();

            if let Some(primary_key_in_clause) = &self.internal_clauses.primary_key_in_clause {
                query.insert_keys(
                    primary_key_in_clause
                        .iter()
                        .map(|identifier| identifier.to_vec())
                        .collect(),
                );

                Ok(PathQuery::new(
                    path,
                    SizedQuery::new(query, Some(primary_key_in_clause.len() as u16), None),
                ))
            } else {
                query.insert_all();

                Ok(PathQuery::new(path, SizedQuery::new(query, None, None)))
            }
        }
    }

    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_raw_results_no_proof(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u64), Error> {
        let mut drive_operations = vec![];
        let (items, _skipped) = self.execute_raw_results_no_proof_internal(
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
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
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
        let path_query = self.construct_path_query(platform_version)?;
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
}
