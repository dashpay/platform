/// resolver module
pub mod resolve;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::object_size_info::{DataContractOwnedResolvedInfo, DataContractResolvedInfo};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef, Index};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::serialization::PlatformSerializable;
use dpp::util::hash::hash_double;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::ProtocolError;

/// Represents information related to a contested document resource vote poll, along with
/// associated contract details.
///
/// This structure holds a reference to the contract, the document type name,
/// the index name, and the index values used for the poll.
#[derive(Debug, PartialEq, Clone)]
pub struct ContestedDocumentResourceVotePollWithContractInfo {
    /// The contract information associated with the document.
    pub contract: DataContractOwnedResolvedInfo,
    /// The name of the document type.
    pub document_type_name: String,
    /// The name of the index.
    pub index_name: String,
    /// The values used in the index for the poll.
    pub index_values: Vec<Value>,
}

impl From<ContestedDocumentResourceVotePollWithContractInfo> for ContestedDocumentResourceVotePoll {
    fn from(value: ContestedDocumentResourceVotePollWithContractInfo) -> Self {
        let ContestedDocumentResourceVotePollWithContractInfo {
            contract,
            document_type_name,
            index_name,
            index_values,
        } = value;
        ContestedDocumentResourceVotePoll {
            contract_id: contract.id(),
            document_type_name,
            index_name,
            index_values,
        }
    }
}

impl From<&ContestedDocumentResourceVotePollWithContractInfo>
    for ContestedDocumentResourceVotePoll
{
    fn from(value: &ContestedDocumentResourceVotePollWithContractInfo) -> Self {
        let ContestedDocumentResourceVotePollWithContractInfo {
            contract,
            document_type_name,
            index_name,
            index_values,
        } = value;
        ContestedDocumentResourceVotePoll {
            contract_id: contract.id(),
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values.clone(),
        }
    }
}

/// Represents information related to a contested document resource vote poll, along with
/// associated contract details.
///
/// This structure holds a reference to the contract, the document type name,
/// the index name, and the index values used for the poll.
#[derive(Debug, PartialEq, Clone)]
pub struct ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a> {
    /// The contract information associated with the document.
    pub contract: DataContractResolvedInfo<'a>,
    /// The name of the document type.
    pub document_type_name: String,
    /// The name of the index.
    pub index_name: String,
    /// The values used in the index for the poll.
    pub index_values: Vec<Value>,
}

impl<'a> From<&'a ContestedDocumentResourceVotePollWithContractInfo>
    for ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>
{
    fn from(value: &'a ContestedDocumentResourceVotePollWithContractInfo) -> Self {
        let ContestedDocumentResourceVotePollWithContractInfo {
            contract,
            document_type_name,
            index_name,
            index_values,
        } = value;
        ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: contract.into(),
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values.clone(),
        }
    }
}

impl<'a> From<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>>
    for ContestedDocumentResourceVotePoll
{
    fn from(value: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed) -> Self {
        let ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract,
            document_type_name,
            index_name,
            index_values,
        } = value;

        ContestedDocumentResourceVotePoll {
            contract_id: contract.id(),
            document_type_name,
            index_name,
            index_values,
        }
    }
}

impl<'a> From<&ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>>
    for ContestedDocumentResourceVotePoll
{
    fn from(value: &ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed) -> Self {
        let ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract,
            document_type_name,
            index_name,
            index_values,
        } = value;

        ContestedDocumentResourceVotePoll {
            contract_id: contract.id(),
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values.clone(),
        }
    }
}

impl ContestedDocumentResourceVotePollWithContractInfo {
    /// Serializes the contested document resource vote poll with contract information to bytes.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<u8>` with the serialized bytes, or a `ProtocolError` if serialization fails.
    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let contested_document_resource_vote_poll: ContestedDocumentResourceVotePoll = self.into();
        contested_document_resource_vote_poll.serialize_to_bytes()
    }

    /// Computes the double SHA-256 hash of the serialized contested document resource vote poll.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `[u8; 32]` array with the hash, or a `ProtocolError` if hashing fails.
    pub fn sha256_2_hash(&self) -> Result<[u8; 32], ProtocolError> {
        let encoded = self.serialize_to_bytes()?;
        Ok(hash_double(encoded))
    }

    /// Retrieves the specialized balance identifier associated with the contested document resource vote poll.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Identifier`, or a `ProtocolError` if retrieving the balance ID fails.
    pub fn specialized_balance_id(&self) -> Result<Identifier, ProtocolError> {
        self.unique_id()
    }

    /// Retrieves the unique identifier associated with the contested document resource vote poll.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Identifier`, or a `ProtocolError` if retrieving the unique ID fails.
    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(Identifier::new)
    }
}

impl<'a> ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a> {
    /// Serializes the contested document resource vote poll with contract information (allowing borrowed data) to bytes.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<u8>` with the serialized bytes, or a `ProtocolError` if serialization fails.
    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let contested_document_resource_vote_poll: ContestedDocumentResourceVotePoll = self.into();
        contested_document_resource_vote_poll.serialize_to_bytes()
    }

    /// Computes the double SHA-256 hash of the serialized contested document resource vote poll (allowing borrowed data).
    ///
    /// # Returns
    ///
    /// A `Result` containing a `[u8; 32]` array with the hash, or a `ProtocolError` if hashing fails.
    pub fn sha256_2_hash(&self) -> Result<[u8; 32], ProtocolError> {
        let encoded = self.serialize_to_bytes()?;
        Ok(hash_double(encoded))
    }

    /// Retrieves the specialized balance identifier associated with the contested document resource vote poll (allowing borrowed data).
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Identifier`, or a `ProtocolError` if retrieving the balance ID fails.
    pub fn specialized_balance_id(&self) -> Result<Identifier, ProtocolError> {
        self.unique_id()
    }

    /// Retrieves the unique identifier associated with the contested document resource vote poll (allowing borrowed data).
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Identifier`, or a `ProtocolError` if retrieving the unique ID fails.
    pub fn unique_id(&self) -> Result<Identifier, ProtocolError> {
        self.sha256_2_hash().map(Identifier::new)
    }
}

impl ContestedDocumentResourceVotePollWithContractInfo {
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

impl<'a> ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a> {
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
