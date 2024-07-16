use super::ContractLookupFn;
use crate::drive::votes::paths::vote_contested_resource_contract_documents_indexes_path_vec;
#[cfg(feature = "server")]
use crate::drive::Drive;
use crate::error::contract::DataContractError;
#[cfg(feature = "server")]
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::GroveError;
use crate::query::Query;
use crate::util::object_size_info::DataContractResolvedInfo;
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{DocumentTypeRef, Index, IndexProperty};
use dpp::data_contract::DataContract;
#[cfg(feature = "server")]
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;

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
    pub start_at_value: Option<(Value, bool)>,
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
    pub start_at_value: &'a Option<(Value, bool)>,
    /// Limit
    pub limit: Option<u16>,
    /// Ascending
    pub order_ascending: bool,
}

impl VotePollsByDocumentTypeQuery {
    #[cfg(feature = "server")]
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
            contract_id,
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit,
            order_ascending,
        } = self;
        let contract = drive
            .fetch_contract(
                contract_id.to_buffer(),
                None,
                None,
                transaction,
                platform_version,
            )
            .unwrap()?
            .ok_or(Error::DataContract(DataContractError::MissingContract(
                "data contract not found when resolving vote polls by document type query"
                    .to_string(),
            )))?;

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
        known_contracts_provider_fn: &ContractLookupFn,
    ) -> Result<ResolvedVotePollsByDocumentTypeQuery, Error> {
        let VotePollsByDocumentTypeQuery {
            contract_id,
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit,
            order_ascending,
        } = self;
        let contract = known_contracts_provider_fn(contract_id)?.ok_or(Error::DataContract(
            DataContractError::MissingContract(format!(
                "data contract with id {} can not be provided",
                contract_id
            )),
        ))?;

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
            contract_id,
            document_type_name,
            index_name,
            start_index_values,
            end_index_values,
            start_at_value,
            limit,
            order_ascending,
        } = self;
        if contract_id != data_contract.id_ref() {
            return Err(Error::DataContract(
                DataContractError::ProvidedContractMismatch(format!(
                    "data contract provided {} is not the one required {}",
                    data_contract.id_ref(),
                    contract_id
                )),
            ));
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
    ) -> Result<(Vec<Value>, Credits), Error> {
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
                None,
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
    pub(crate) fn document_type(&self) -> Result<DocumentTypeRef, Error> {
        Ok(self
            .contract
            .as_ref()
            .document_type_for_name(self.document_type_name.as_str())?)
    }
    pub(crate) fn index(&self) -> Result<&Index, Error> {
        let index = self
            .contract
            .as_ref()
            .document_type_borrowed_for_name(self.document_type_name.as_str())?
            .find_contested_index()
            .ok_or(Error::Query(QuerySyntaxError::UnknownIndex(format!(
                "document type {} does not have a contested index",
                self.document_type_name.as_str()
            ))))?;
        if index.name.as_str() != self.index_name.as_str() {
            return Err(Error::Query(QuerySyntaxError::UnknownIndex(format!(
                "index with name {} is not the contested index on the document type {}, {} is the name of the only contested index (contested resources query)",
                self.index_name.as_str(), self.document_type_name.as_str(),  &index.name
            ))));
        }
        Ok(index)
    }

    /// Creates the vectors of indexes
    fn indexes_vectors<'b>(
        &self,
        index: &'b Index,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, Vec<Vec<u8>>, &'b IndexProperty), Error> {
        let document_type = self.document_type()?;
        let properties_iter = index.properties.iter();
        let mut start_values_iter = self.start_index_values.iter();
        let has_end_index_values = !self.end_index_values.is_empty();
        let mut end_values_iter = self.end_index_values.iter();
        let mut start_values_vec = vec![];
        let mut end_values_vec = vec![];
        let mut ended_start_values = false;
        let mut middle_index_property = None;
        for index_property in properties_iter {
            if !ended_start_values {
                if let Some(start_value) = start_values_iter.next() {
                    let encoded = document_type.serialize_value_for_key(
                        &index_property.name,
                        start_value,
                        platform_version,
                    )?;
                    start_values_vec.push(encoded);
                } else {
                    ended_start_values = true;
                    middle_index_property = Some(index_property);
                }
            } else if let Some(end_value) = end_values_iter.next() {
                let encoded = document_type.serialize_value_for_key(
                    &index_property.name,
                    end_value,
                    platform_version,
                )?;
                end_values_vec.push(encoded);
            } else {
                break;
            }
        }
        if end_values_iter.next().is_some() {
            return Err(Error::Query(QuerySyntaxError::IndexValuesError(
                "too many end index values were provided".to_string(),
            )));
        }
        let middle_index_property = middle_index_property.ok_or_else(|| {
            let error_msg = if has_end_index_values {
                "since end index values were provided, the start index values and the end index values must be equal to the amount of properties in the contested index minus one, we could not find a middle property".to_string()
            } else {
                "too many start index values were provided, since no end index values were provided, the start index values must be less than the amount of properties in the contested index".to_string()
            };
            Error::Query(QuerySyntaxError::IndexValuesError(error_msg))
        })?;
        Ok((start_values_vec, end_values_vec, middle_index_property))
    }

    pub(crate) fn property_name_being_searched(
        &self,
        index: &'a Index,
    ) -> Result<&'a IndexProperty, Error> {
        let offset = self.start_index_values.len();
        index
            .properties
            .get(offset)
            .ok_or(Error::Query(QuerySyntaxError::IndexValuesError(format!(
            "there are too many start index values to be able to make a search max is {}, got {}",
            index.properties.len() - 1,
            offset
        ))))
    }

    pub(crate) fn result_is_in_key(&self) -> bool {
        // this means that the keys are the values that we are interested in
        self.end_index_values.is_empty()
    }

    pub(crate) fn result_path_index(&self) -> usize {
        // 6 because of:
        // voting sub tree (112)
        // contested ('c')
        // voting part
        // contract id
        // document type name
        // 1
        6 + self.start_index_values.len()
    }

    /// Operations to construct a path query.
    pub(crate) fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let index = self.index()?;
        self.construct_path_query_with_known_index(index, platform_version)
    }

    /// Operations to construct a path query.
    pub(crate) fn construct_path_query_with_known_index(
        &self,
        index: &Index,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let mut path = vote_contested_resource_contract_documents_indexes_path_vec(
            self.contract.id().as_ref(),
            self.document_type_name,
        );

        let (mut start, end, middle_property) = self.indexes_vectors(index, platform_version)?;

        if !start.is_empty() {
            path.append(&mut start);
        }

        let mut query = Query::new_with_direction(self.order_ascending);

        // this is a range on all elements
        match &self.start_at_value {
            None => {
                query.insert_all();
            }
            Some((starts_at_key_bytes, start_at_included)) => {
                let starts_at_key = self.document_type()?.serialize_value_for_key(
                    &middle_property.name,
                    starts_at_key_bytes,
                    platform_version,
                )?;

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

        if !end.is_empty() {
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
        let index = self.index()?;
        let path_query = self.construct_path_query_with_known_index(index, platform_version)?;
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
                let property_name_being_searched = self.property_name_being_searched(index)?;
                query_result_elements.to_path_key_elements()
                    .into_iter()
                    .map(|(mut path, key, _)| {
                        if result_is_in_key {
                            // the result is in the key because we did not provide any end index values
                            // like this  <------ start index values (path) --->    Key
                            // properties ------- --------- --------- ----------  ------- 
                            document_type.deserialize_value_for_key(property_name_being_searched.name.as_str(), key.as_slice(), platform_version).map_err(Error::Protocol)
                        } else if path.len() < result_path_index.unwrap() {

                            Err(Error::Drive(DriveError::CorruptedCodeExecution("the path length should always be bigger or equal to the result path index")))
                        } else {
                            // the result is in the path because we did provide end index values
                            // like this  <------ start index values (path) --->    Key
                            // properties ------- --------- --------- ----------  ------- 
                            let inner_path_value_bytes = path.remove(result_path_index.unwrap());
                            document_type.deserialize_value_for_key(property_name_being_searched.name.as_str(), inner_path_value_bytes.as_slice(), platform_version).map_err(Error::Protocol)
                        }
                    }).collect::<Result<Vec<Value>, Error>>()
            }
        }
    }
}
