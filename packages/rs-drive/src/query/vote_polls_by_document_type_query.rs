use crate::common::encode::{decode_u64, encode_u64};
use crate::drive::votes::paths::{vote_contested_resource_active_polls_tree_path_vec, vote_contested_resource_end_date_queries_tree_path_vec};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::{GroveError, Query};
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::TimestampMillis;
use dpp::serialization::PlatformDeserializable;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::{PathQuery, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::sync::Arc;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef, Index};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use crate::drive::object_size_info::DataContractResolvedInfo;
use crate::error::contract::DataContractError;
use crate::error::query::QuerySyntaxError;
use crate::query::vote_poll_vote_state_query::{ContestedDocumentVotePollDriveQuery, ResolvedContestedDocumentVotePollDriveQuery};

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct VotePollsByDocumentTypeQuery {
    /// The contract information associated with the document.
    pub contract_id: Identifier,
    /// The name of the document type.
    pub document_type_name: String,
    /// The name of the index.
    pub index_name: String,
    /// All values that are before the missing property number
    pub start_index_values: Vec<Value>,
    /// All values that are after the missing property number
    pub end_index_values: Vec<Value>,
    /// Start at value
    pub start_at_value: Option<(Vec<u8>, bool)>,
    /// Limit
    pub limit: Option<u16>,
    /// Ascending
    pub order_ascending: bool,
}

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ResolvedVotePollsByDocumentTypeQuery<'a> {
    /// What vote poll are we asking for?
    pub contract: DataContractResolvedInfo<'a>,
    /// The name of the document type.
    pub document_type_name: &'a String,
    /// The name of the index.
    pub index_name: &'a String,
    /// All values that are before the missing property number
    pub start_index_values: &'a Vec<Value>,
    /// All values that are after the missing property number
    pub end_index_values: &'a Vec<Value>,
    /// Start at value
    pub start_at_value: &'a Option<(Vec<u8>, bool)>,
    /// Limit
    pub limit: Option<u16>,
    /// Ascending
    pub order_ascending: bool,
}


impl VotePollsByDocumentTypeQuery {
    /// Resolves the contested document vote poll drive query.
    ///
    /// This method processes the query by interacting with the drive, using the provided
    /// transaction and platform version to ensure consistency and compatibility.
    ///
    /// # Parameters
    ///
    /// * `drive`: A reference to the `Drive` object used for database interactions.
    /// * `transaction`: The transaction argument used to ensure consistency during the resolve operation.
    /// * `platform_version`: The platform version to ensure compatibility.
    ///
    /// # Returns
    ///
    /// * `Ok(ResolvedContestedDocumentVotePollDriveQuery)` - The resolved query information.
    /// * `Err(Error)` - An error if the resolution process fails.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` variant if there is an issue resolving the query.
    /// The specific error depends on the underlying problem encountered during resolution.
    pub fn resolve<'a>(
        &'a self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedVotePollsByDocumentTypeQuery<'a>, Error> {
        let VotePollsByDocumentTypeQuery {
            contract_id, document_type_name, index_name, start_index_values, end_index_values, start_at_value, limit, order_ascending
        } = self;
        let contract = drive.fetch_contract(contract_id.to_buffer(), None, None, transaction, platform_version).unwrap()?.ok_or(Error::DataContract(DataContractError::MissingContract("data contract not found when trying to resolve contested document resource vote poll".to_string())))?;

        Ok(ResolvedVotePollsByDocumentTypeQuery {
            contract: DataContractResolvedInfo::ArcDataContractFetchInfo(contract),
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit: *limit,
            order_ascending: *order_ascending,
        })
    }

    /// Resolves with a known contract provider
    pub fn resolve_with_known_contracts_provider(
        &self,
        known_contracts_provider_fn: &impl Fn(&Identifier) -> Result<Option<Arc<DataContract>>, Error>,
    ) -> Result<ResolvedVotePollsByDocumentTypeQuery, Error> {
        let VotePollsByDocumentTypeQuery {
            contract_id, document_type_name, index_name, start_index_values, end_index_values, start_at_value, limit, order_ascending
        } = self;
        let contract = known_contracts_provider_fn(contract_id)?.ok_or(Error::DataContract(DataContractError::MissingContract(format!("data contract with id {} can not be provided", contract_id))))?;

        Ok(ResolvedVotePollsByDocumentTypeQuery {
            contract: DataContractResolvedInfo::ArcDataContract(contract),
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit: *limit,
            order_ascending: *order_ascending,
        })
    }

    /// Resolves with a provided borrowed contract
    pub fn resolve_with_provided_borrowed_contract<'a>(
        &'a self,
        data_contract: &'a DataContract,
    ) -> Result<ResolvedVotePollsByDocumentTypeQuery<'a>, Error> {
        let VotePollsByDocumentTypeQuery {
            contract_id, document_type_name, index_name, start_index_values, end_index_values, start_at_value, limit, order_ascending
        } = self;
        if contract_id != data_contract.id_ref() {
            return Err(Error::DataContract(DataContractError::ProvidedContractMismatch(format!("data contract provided {} is not the one required {}", data_contract.id_ref(), contract_id))));
        }
        Ok(ResolvedVotePollsByDocumentTypeQuery {
            contract: DataContractResolvedInfo::BorrowedDataContract(data_contract),
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit: *limit,
            order_ascending: *order_ascending,
        })
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with proof and returns the items.
    pub fn execute_with_proof(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        resolved.execute_with_proof(drive, transaction, drive_operations, platform_version)
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
            Vec<Value>,
            Credits,
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
    ) -> Result<Vec<Value>, Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        resolved.execute_no_proof(drive, transaction, drive_operations, platform_version)
    }
}

impl<'a> ResolvedVotePollsByDocumentTypeQuery<'a> {
    fn document_type(&self) -> Result<DocumentTypeRef, Error> {
        Ok(self.contract.as_ref().document_type_for_name(self.document_type_name.as_str())?)
    }
    fn index(&self) -> Result<&Index, Error> {
        let index = self.contract.as_ref().document_type_borrowed_for_name(self.document_type_name.as_str())?.find_contested_index().ok_or(
            Error::Query(QuerySyntaxError::UnknownIndex(format!(
                "document type {} does not have a contested index",
                self.document_type_name.as_str()
            )))
        )?;
        if index.name.as_str() != self.index_name.as_str() {
            return Err(Error::Query(QuerySyntaxError::UnknownIndex(format!(
                "index with name {} is not the contested index on the document type {}, {} is the name of the only contested index (contested resources query)",
                self.index_name.as_str(), self.document_type_name.as_str(),  &index.name
            ))));
        }
        Ok(index)
    }
    fn indexes_vectors(&self, platform_version: &PlatformVersion) -> Result<(Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<u8>), Error> {
        let document_type = self.document_type()?;
        let index = self.index()?;
        let mut properties_iter = index.properties.iter();
        let mut start_values_iter = self.start_index_values.iter();
        let mut end_values_iter = self.end_index_values.iter();
        let mut start_values_vec = vec![];
        let mut end_values_vec = vec![];
        let mut ended_start_values = false;
        let mut started_end_values = false;
        while let Some(index_property) = properties_iter.next() {
            if !ended_start_values {
                if let Some(start_value) = start_values_iter.next() {
                    let encoded = document_type.serialize_value_for_key(&index_property.name, start_value, platform_version)?;
                    start_values_vec.push(encoded);
                } else {
                    ended_start_values = true;
                }
            } else if started_end_values {
                if let Some(end_value) = end_values_iter.next() {
                    let encoded = document_type.serialize_value_for_key(&index_property.name, end_value, platform_version)?;
                    end_values_vec.push(encoded);
                } else {
                    return Err(Error::Query(QuerySyntaxError::MissingIndexValues("the start index values and the end index values must be equal to the amount of properties in the contested index minus one".to_string())))
                }
            } else {
                started_end_values = true;
            }
        }
        Ok((start_values_vec, end_values_vec))
    }
    
    pub fn result_is_in_key(&self) -> bool {
        // this means that the keys are the values that we are interested in
        self.end_index_values.is_empty()
    }
    
    fn result_path_index(&self) -> usize {
        5 + self.start_index_values.len()
    }
    
    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let mut path = vote_contested_resource_active_polls_tree_path_vec();
        
        let (mut start, end) = self.indexes_vectors(platform_version)?;

        if !start.is_empty() {
            path.append(&mut start);
        }

        let mut query = Query::new_with_direction(self.order_ascending);
        
        if !end.is_empty(){
            query.default_subquery_branch.subquery_path = Some(end);
        }

        Ok(PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: self.limit,
                offset: None,
            },
        })
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with proof and returns the items.
    pub fn execute_with_proof(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = self.construct_path_query(platform_version)?;
        drive.grove_get_proved_path_query(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Value>, Error> {
        let path_query = self.construct_path_query(platform_version)?;
        let query_result = drive.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok(vec![]),
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => {
                let result_is_in_key = self.result_is_in_key();
                let result_path_index = if result_is_in_key {
                    None
                } else {
                    Some(self.result_path_index())
                };
                let document_type = self.document_type()?;
                query_result_elements.to_path_key_elements()
                    .into_iter()
                    .map(|(mut path, key, _)| {
                        if result_is_in_key {
                            document_type.deserialize_value_for_key()
                            Ok(key)
                        } else if path.len() < result_path_index.unwrap() {
                            
                            Err()
                        } else {
                            Ok(path.remove(result_path_index.unwrap()))
                        }
                    }).collect::<Result<Vec<u8>, Error>>()

            }
        }
    }
}
