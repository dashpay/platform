use crate::drive::contract::DataContractFetchInfo;
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
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub struct ContestedDocumentResourceVotePollWithContractInfo {
    pub contract: Arc<DataContractFetchInfo>,
    pub document_type_name: String,
    pub index_name: String,
    pub index_values: Vec<Value>,
}

impl ContestedDocumentResourceVotePollWithContractInfo {
    pub fn index(&self) -> Result<&Index, Error> {
        self.contract.contract.document_type_borrowed_for_name(self.document_type_name.as_str())?.indexes().get(&self.index_name).ok_or(Error::Drive(DriveError::ContestedIndexNotFound("contested index not found when try to get it from the contested document resource vote poll with contract info")))
    }

    pub fn document_type(&self) -> Result<DocumentTypeRef, Error> {
        self.contract
            .contract
            .document_type_for_name(self.document_type_name.as_str())
            .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))
    }

    pub fn document_type_borrowed(&self) -> Result<&DocumentType, Error> {
        self.contract
            .contract
            .document_type_borrowed_for_name(self.document_type_name.as_str())
            .map_err(|e| Error::Protocol(ProtocolError::DataContractError(e)))
    }
}

pub trait ContestedDocumentResourceVotePollResolver {
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
            contract,
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values.clone(),
        })
    }
}
