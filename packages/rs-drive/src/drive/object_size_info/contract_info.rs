use crate::drive::contract::DataContractFetchInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
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
