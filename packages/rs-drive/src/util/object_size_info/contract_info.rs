#[cfg(feature = "server")]
use crate::drive::contract::DataContractFetchInfo;
#[cfg(feature = "server")]
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::error::document::DocumentError;
#[cfg(feature = "server")]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
#[cfg(feature = "server")]
use dpp::ProtocolError;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
#[cfg(feature = "server")]
use platform_version::version::PlatformVersion;
use std::sync::Arc;

/// Represents various forms of accessing or representing a data contract.
/// This enum is used to handle different scenarios in which data contracts
/// might be needed, providing a unified interface to access their data.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
#[cfg(feature = "server")]
pub enum DataContractInfo<'a> {
    /// A unique identifier for a data contract. This variant is typically used
    /// when only the identity of the data contract is required without needing
    /// to access the full contract itself.
    DataContractId(Identifier),

    #[cfg(feature = "server")]
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

#[cfg(feature = "server")]
impl<'a> DataContractInfo<'a> {
    #[cfg(feature = "server")]
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
                Ok(DataContractResolvedInfo::ArcDataContractFetchInfo(
                    contract_fetch_info,
                ))
            }
            DataContractInfo::DataContractFetchInfo(contract_fetch_info) => Ok(
                DataContractResolvedInfo::ArcDataContractFetchInfo(contract_fetch_info),
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
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum DataContractOwnedResolvedInfo {
    #[cfg(feature = "server")]
    /// Information necessary for fetched data contracts, encapsulated in an
    /// `Arc` to ensure thread-safe shared ownership and access.
    DataContractFetchInfo(Arc<DataContractFetchInfo>),

    /// An owned instance of a data contract. This variant provides full control
    /// and mutability over the data contract, suitable for scenarios requiring
    /// modifications or extended operations on the data contract.
    OwnedDataContract(DataContract),
}

impl DataContractOwnedResolvedInfo {
    /// The id of the contract
    pub fn id(&self) -> Identifier {
        match self {
            #[cfg(feature = "server")]
            DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info) => {
                fetch_info.contract.id()
            }
            DataContractOwnedResolvedInfo::OwnedDataContract(data_contract) => data_contract.id(),
        }
    }
}
impl AsRef<DataContract> for DataContractOwnedResolvedInfo {
    /// The ref of the contract
    fn as_ref(&self) -> &DataContract {
        match self {
            #[cfg(feature = "server")]
            DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info) => {
                &fetch_info.contract
            }
            DataContractOwnedResolvedInfo::OwnedDataContract(owned) => owned,
        }
    }
}

impl DataContractOwnedResolvedInfo {
    /// Get the contract as owned
    pub fn into_owned(self) -> DataContract {
        match self {
            #[cfg(feature = "server")]
            DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info) => {
                fetch_info.contract.clone()
            }
            DataContractOwnedResolvedInfo::OwnedDataContract(owned) => owned,
        }
    }
}

/// Contains resolved data contract information, typically used after initial
/// fetching or retrieval steps have been completed. This enum simplifies handling
/// of data contract states post-retrieval.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum DataContractResolvedInfo<'a> {
    #[cfg(feature = "server")]
    /// Information necessary for fetched data contracts, encapsulated in an
    /// `Arc` to ensure thread-safe shared ownership and access.
    ArcDataContractFetchInfo(Arc<DataContractFetchInfo>),

    /// Arc Data contract
    ArcDataContract(Arc<DataContract>),

    /// A borrowed reference to a resolved data contract. This variant is suitable
    /// for scenarios where temporary, read-only access to a data contract is required.
    BorrowedDataContract(&'a DataContract),

    /// An owned instance of a data contract. This variant provides full control
    /// and mutability over the data contract, suitable for scenarios requiring
    /// modifications or extended operations on the data contract.
    OwnedDataContract(DataContract),
}

impl<'a> From<&'a DataContractOwnedResolvedInfo> for DataContractResolvedInfo<'a> {
    fn from(value: &'a DataContractOwnedResolvedInfo) -> Self {
        match value {
            #[cfg(feature = "server")]
            DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info) => {
                DataContractResolvedInfo::ArcDataContractFetchInfo(fetch_info.clone())
            }
            DataContractOwnedResolvedInfo::OwnedDataContract(data_contract) => {
                DataContractResolvedInfo::BorrowedDataContract(data_contract)
            }
        }
    }
}

impl DataContractResolvedInfo<'_> {
    /// The id of the contract
    pub fn id(&self) -> Identifier {
        match self {
            #[cfg(feature = "server")]
            DataContractResolvedInfo::ArcDataContractFetchInfo(fetch_info) => {
                fetch_info.contract.id()
            }
            DataContractResolvedInfo::BorrowedDataContract(data_contract) => data_contract.id(),
            DataContractResolvedInfo::OwnedDataContract(data_contract) => data_contract.id(),
            DataContractResolvedInfo::ArcDataContract(data_contract) => data_contract.id(),
        }
    }
}
impl AsRef<DataContract> for DataContractResolvedInfo<'_> {
    /// The ref of the contract
    fn as_ref(&self) -> &DataContract {
        match self {
            #[cfg(feature = "server")]
            DataContractResolvedInfo::ArcDataContractFetchInfo(fetch_info) => &fetch_info.contract,
            DataContractResolvedInfo::BorrowedDataContract(borrowed) => borrowed,
            DataContractResolvedInfo::OwnedDataContract(owned) => owned,
            DataContractResolvedInfo::ArcDataContract(data_contract) => data_contract.as_ref(),
        }
    }
}

/// Enumerates methods for identifying or referencing document types, accommodating various application needs.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
#[cfg(feature = "server")]
pub enum DocumentTypeInfo<'a> {
    /// Contains the document type name as an owned `String`, suitable for dynamic or mutable scenarios.
    DocumentTypeName(String),

    /// References the document type name via a borrowed `&'a str`, ideal for static or temporary usage.
    DocumentTypeNameAsStr(&'a str),

    /// References a document type that has already been resolved through `DocumentTypeRef`.
    DocumentTypeRef(DocumentTypeRef<'a>),
}

#[cfg(feature = "server")]
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
