#[cfg(feature = "server")]
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
use crate::drive::Drive;
use crate::error::contract::DataContractError;
use crate::error::Error;
#[cfg(feature = "verify")]
use crate::query::ContractLookupFn;
use crate::util::object_size_info::DataContractOwnedResolvedInfo;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::util::object_size_info::DataContractResolvedInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::prelude::DataContract;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
#[cfg(any(feature = "server", feature = "verify"))]
use std::sync::Arc;

/// A trait for resolving information related to a contested document resource vote poll.
///
/// This trait defines a method to resolve and retrieve the necessary contract and index
/// information associated with a contested document resource vote poll.
pub trait ContestedDocumentResourceVotePollResolver {
    #[cfg(feature = "server")]
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
    #[cfg(feature = "server")]

    /// Resolve owned
    fn resolve_owned(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfo, Error>;

    #[cfg(feature = "server")]
    /// Resolve into a struct that allows for a borrowed contract
    fn resolve_allow_borrowed<'a>(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error>;

    #[cfg(feature = "verify")]

    /// Resolves into a struct, the contract itself will be held with Arc
    fn resolve_with_known_contracts_provider<'a>(
        &self,
        known_contracts_provider_fn: &ContractLookupFn,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error>;

    #[cfg(any(feature = "verify", feature = "server"))]
    /// Resolve by providing the contract
    fn resolve_with_provided_borrowed_contract<'a>(
        &self,
        data_contract: &'a DataContract,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error>;

    #[cfg(feature = "server")]
    /// Resolve owned into a struct that allows for a borrowed contract
    fn resolve_owned_allow_borrowed<'a>(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error>;

    /// Resolve owned into a struct that allows for a borrowed contract
    #[cfg(feature = "server")]
    fn resolve_with_provided_arc_contract_fetch_info(
        &self,
        data_contract: Arc<DataContractFetchInfo>,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfo, Error>;

    /// Resolve owned into a struct that allows for a borrowed contract
    #[cfg(feature = "server")]
    fn resolve_owned_with_provided_arc_contract_fetch_info(
        self,
        data_contract: Arc<DataContractFetchInfo>,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfo, Error>;
}

impl ContestedDocumentResourceVotePollResolver for ContestedDocumentResourceVotePoll {
    #[cfg(feature = "server")]
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
            contract: DataContractOwnedResolvedInfo::DataContractFetchInfo(contract),
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values.clone(),
        })
    }

    #[cfg(feature = "server")]
    fn resolve_owned(
        self,
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

        let contract = drive.fetch_contract(contract_id.to_buffer(), None, None, transaction, platform_version).unwrap()?.ok_or(Error::DataContract(DataContractError::MissingContract("data contract not found when trying to resolve contested document resource vote poll as owned".to_string())))?;
        Ok(ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::DataContractFetchInfo(contract),
            document_type_name,
            index_name,
            index_values,
        })
    }

    #[cfg(feature = "server")]
    fn resolve_allow_borrowed<'a>(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error> {
        let ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        } = self;

        let contract = drive.fetch_contract(contract_id.to_buffer(), None, None, transaction, platform_version).unwrap()?.ok_or(Error::DataContract(DataContractError::MissingContract("data contract not found when trying to resolve contested document resource vote poll as borrowed".to_string())))?;
        Ok(
            ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                contract: DataContractResolvedInfo::ArcDataContractFetchInfo(contract),
                document_type_name: document_type_name.clone(),
                index_name: index_name.clone(),
                index_values: index_values.clone(),
            },
        )
    }

    #[cfg(feature = "verify")]
    fn resolve_with_known_contracts_provider<'a>(
        &self,
        known_contracts_provider_fn: &ContractLookupFn,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error> {
        let ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        } = self;

        let contract = known_contracts_provider_fn(contract_id)?.ok_or(Error::DataContract(
            DataContractError::MissingContract(format!(
                "data contract with id {} can not be provided",
                contract_id
            )),
        ))?;
        Ok(
            ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                contract: DataContractResolvedInfo::ArcDataContract(contract),
                document_type_name: document_type_name.clone(),
                index_name: index_name.clone(),
                index_values: index_values.clone(),
            },
        )
    }

    #[cfg(feature = "server")]
    fn resolve_with_provided_arc_contract_fetch_info(
        &self,
        data_contract: Arc<DataContractFetchInfo>,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfo, Error> {
        let ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        } = self;

        if contract_id != data_contract.contract.id_ref() {
            return Err(Error::DataContract(
                DataContractError::ProvidedContractMismatch(format!(
                    "data contract provided {} is not the one required {}",
                    data_contract.contract.id_ref(),
                    contract_id
                )),
            ));
        }
        Ok(ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::DataContractFetchInfo(data_contract),
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values.clone(),
        })
    }

    #[cfg(feature = "server")]
    fn resolve_owned_with_provided_arc_contract_fetch_info(
        self,
        data_contract: Arc<DataContractFetchInfo>,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfo, Error> {
        let ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        } = self;

        if contract_id != data_contract.contract.id_ref() {
            return Err(Error::DataContract(
                DataContractError::ProvidedContractMismatch(format!(
                    "data contract provided {} is not the one required {}",
                    data_contract.contract.id_ref(),
                    contract_id
                )),
            ));
        }
        Ok(ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::DataContractFetchInfo(data_contract),
            document_type_name,
            index_name,
            index_values,
        })
    }

    #[cfg(any(feature = "verify", feature = "server"))]
    fn resolve_with_provided_borrowed_contract<'a>(
        &self,
        data_contract: &'a DataContract,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error> {
        let ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
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
        Ok(
            ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                contract: DataContractResolvedInfo::BorrowedDataContract(data_contract),
                document_type_name: document_type_name.clone(),
                index_name: index_name.clone(),
                index_values: index_values.clone(),
            },
        )
    }

    #[cfg(feature = "server")]
    fn resolve_owned_allow_borrowed<'a>(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>, Error> {
        let ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        } = self;

        let contract = drive.fetch_contract(contract_id.to_buffer(), None, None, transaction, platform_version).unwrap()?.ok_or(Error::DataContract(DataContractError::MissingContract("data contract not found when trying to resolve contested document resource vote poll as owned, but allowing borrowed".to_string())))?;
        Ok(
            ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                contract: DataContractResolvedInfo::ArcDataContractFetchInfo(contract),
                document_type_name,
                index_name,
                index_values,
            },
        )
    }
}
