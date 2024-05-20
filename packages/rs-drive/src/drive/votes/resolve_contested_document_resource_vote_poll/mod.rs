use crate::drive::object_size_info::DataContractResolvedInfo;
use crate::drive::Drive;
use crate::error::contract::DataContractError;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef, Index};
use dpp::platform_value::Value;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

/// Represents information related to a contested document resource vote poll, along with
/// associated contract details.
///
/// This structure holds a reference to the contract, the document type name,
/// the index name, and the index values used for the poll.
#[derive(Debug, PartialEq, Clone)]
pub struct ContestedDocumentResourceVotePollWithContractInfo<'a> {
    /// The contract information associated with the document.
    pub contract: DataContractResolvedInfo<'a>,
    /// The name of the document type.
    pub document_type_name: String,
    /// The name of the index.
    pub index_name: String,
    /// The values used in the index for the poll.
    pub index_values: Vec<Value>,
}

impl<'a> ContestedDocumentResourceVotePollWithContractInfo<'a> {
    /// Retrieves the index associated with the document type and index name.
    ///
    /// # Returns
    ///
    /// * `Ok(&Index)` - A reference to the index if found.
    /// * `Err(Error)` - An error if the index is not found or if there is an issue retrieving it.
    ///
    /// # Errors
    ///
    /// This method returns an `Error::Drive` variant with `DriveError::ContestedIndexNotFound`
    /// if the index cannot be found within the document type.
    pub fn index(&self) -> Result<&Index, Error> {
        self.contract
            .as_ref()
            .document_type_borrowed_for_name(self.document_type_name.as_str())?
            .indexes()
            .get(&self.index_name)
            .ok_or(Error::Drive(DriveError::ContestedIndexNotFound(
                "contested index not found when trying to get it from the contested document resource vote poll with contract info"
            )))
    }

    /// Retrieves the document type reference associated with the document type name.
    ///
    /// # Returns
    ///
    /// * `Ok(DocumentTypeRef)` - A reference to the document type if found.
    /// * `Err(Error)` - An error if the document type cannot be found or if there is an issue retrieving it.
    ///
    /// # Errors
    ///
    /// This method returns an `Error::Protocol` variant with `ProtocolError::DataContractError`
    /// if there is an issue retrieving the document type.
    pub fn document_type(&self) -> Result<DocumentTypeRef, Error> {
        self.contract
            .as_ref()
            .document_type_for_name(self.document_type_name.as_str())
            .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))
    }

    /// Borrows a reference to the document type associated with the document type name.
    ///
    /// # Returns
    ///
    /// * `Ok(&DocumentType)` - A borrowed reference to the document type if found.
    /// * `Err(Error)` - An error if the document type cannot be found or if there is an issue retrieving it.
    ///
    /// # Errors
    ///
    /// This method returns an `Error::Protocol` variant with `ProtocolError::DataContractError`
    /// if there is an issue retrieving the document type.
    pub fn document_type_borrowed(&self) -> Result<&DocumentType, Error> {
        self.contract
            .as_ref()
            .document_type_borrowed_for_name(self.document_type_name.as_str())
            .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))
    }
}

/// A trait for resolving information related to a contested document resource vote poll.
///
/// This trait defines a method to resolve and retrieve the necessary contract and index
/// information associated with a contested document resource vote poll.
pub trait ContestedDocumentResourceVotePollResolver {
    /// Resolves the contested document resource vote poll information.
    ///
    /// This method fetches the contract, document type name, index name, and index values
    /// required to process a contested document resource vote poll.
    ///
    /// # Parameters
    ///
    /// * `drive`: A reference to the `Drive` object used for database interactions.
    /// * `transaction`: The transaction argument used to ensure consistency during the resolve operation.
    /// * `platform_version`: The platform version to ensure compatibility.
    ///
    /// # Returns
    ///
    /// * `Ok(ContestedDocumentResourceVotePollWithContractInfo)` - The resolved information needed for the vote poll.
    /// * `Err(Error)` - An error if the resolution process fails.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` variant if there is an issue resolving the contested document resource vote poll
    /// information. The specific error depends on the underlying problem encountered during resolution.
    fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfo, Error>;
}

impl ContestedDocumentResourceVotePollResolver for ContestedDocumentResourceVotePoll {
    fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfo, Error> {
        let ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        } = self;

        let contract = drive.fetch_contract(contract_id.to_buffer(), None, None, transaction, platform_version).unwrap()?.ok_or(Error::DataContract(DataContractError::MissingContract("data contract not found when trying to resolve contested document resource vote poll".to_string())))?;
        Ok(ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractResolvedInfo::DataContractFetchInfo(contract),
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values.clone(),
        })
    }
}
