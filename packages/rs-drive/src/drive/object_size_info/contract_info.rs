use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::sync::Arc;

/// Represents various forms of accessing or representing a data contract.
/// This enum is used to handle different scenarios in which data contracts
/// might be needed, providing a unified interface to access their data.
#[derive(Clone, Debug)]
pub enum DataContractInfo<'a> {
    /// A unique identifier for a data contract. This variant is typically used
    /// when only the identity of the data contract is required without needing
    /// to access the full contract itself.
    DataContractId(Identifier),

    /// Information necessary for fetching a data contract, encapsulated in an
    /// `Arc` for thread-safe shared ownership. This variant is used when the
    /// data needs to be fetched or is not immediately available.
    DataContractFetchInfo(Arc<DataContractFetchInfo>),

    /// A borrowed reference to a data contract. This variant is used for temporary,
    /// read-only access to a data contract, avoiding ownership transfer.
    BorrowedDataContract(&'a DataContract),

    /// An owned version of a data contract. This variant is used when full ownership
    /// and possibly mutability of the data contract is necessary.
    OwnedDataContract(DataContract),
}

impl<'a> DataContractInfo<'a> {
    /// Resolve the data contract info into an object that contains the data contract
    pub(crate) fn resolve(
        self,
        drive: &Drive,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractResolvedInfo<'a>, Error> {
        match self {
            DataContractInfo::DataContractId(contract_id) => {
                let contract_fetch_info = drive
                    .get_contract_with_fetch_info_and_add_to_operations(
                        contract_id.into_buffer(),
                        Some(&block_info.epoch),
                        true,
                        transaction,
                        drive_operations,
                        platform_version,
                    )?
                    .ok_or(Error::Document(DocumentError::DataContractNotFound))?;
                Ok(DataContractResolvedInfo::DataContractFetchInfo(
                    contract_fetch_info,
                ))
            }
            DataContractInfo::DataContractFetchInfo(contract_fetch_info) => Ok(
                DataContractResolvedInfo::DataContractFetchInfo(contract_fetch_info),
            ),
            DataContractInfo::BorrowedDataContract(contract) => {
                Ok(DataContractResolvedInfo::BorrowedDataContract(contract))
            }
            DataContractInfo::OwnedDataContract(contract) => {
                Ok(DataContractResolvedInfo::OwnedDataContract(contract))
            }
        }
    }
}

/// Contains resolved data contract information, typically used after initial
/// fetching or retrieval steps have been completed. This enum simplifies handling
/// of data contract states post-retrieval.
#[derive(Clone, Debug)]
pub(crate) enum DataContractResolvedInfo<'a> {
    /// Information necessary for fetched data contracts, encapsulated in an
    /// `Arc` to ensure thread-safe shared ownership and access.
    DataContractFetchInfo(Arc<DataContractFetchInfo>),

    /// A borrowed reference to a resolved data contract. This variant is suitable
    /// for scenarios where temporary, read-only access to a data contract is required.
    BorrowedDataContract(&'a DataContract),

    /// An owned instance of a data contract. This variant provides full control
    /// and mutability over the data contract, suitable for scenarios requiring
    /// modifications or extended operations on the data contract.
    OwnedDataContract(DataContract),
}
impl<'a> AsRef<DataContract> for DataContractResolvedInfo<'a> {
    fn as_ref(&self) -> &DataContract {
        match self {
            DataContractResolvedInfo::DataContractFetchInfo(fetch_info) => &fetch_info.contract,
            DataContractResolvedInfo::BorrowedDataContract(borrowed) => borrowed,
            DataContractResolvedInfo::OwnedDataContract(owned) => owned,
        }
    }
}

/// Enumerates methods for identifying or referencing document types, accommodating various application needs.
#[derive(Clone, Debug)]
pub enum DocumentTypeInfo<'a> {
    /// Contains the document type name as an owned `String`, suitable for dynamic or mutable scenarios.
    DocumentTypeName(String),

    /// References the document type name via a borrowed `&'a str`, ideal for static or temporary usage.
    DocumentTypeNameAsStr(&'a str),

    /// References a document type that has already been resolved through `DocumentTypeRef`.
    DocumentTypeRef(DocumentTypeRef<'a>),
}

impl<'a> DocumentTypeInfo<'a> {
    /// Resolve the data contract info into an object that contains the data contract
    pub fn resolve(self, contract: &'a DataContract) -> Result<DocumentTypeRef<'a>, ProtocolError> {
        match self {
            DocumentTypeInfo::DocumentTypeName(document_type_name) => contract
                .document_type_for_name(document_type_name.as_str())
                .map_err(ProtocolError::DataContractError),
            DocumentTypeInfo::DocumentTypeNameAsStr(document_type_name) => contract
                .document_type_for_name(document_type_name)
                .map_err(ProtocolError::DataContractError),
            DocumentTypeInfo::DocumentTypeRef(document_type_ref) => Ok(document_type_ref),
        }
    }
}
