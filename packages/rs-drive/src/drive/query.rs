use grovedb::TransactionArg;

use crate::contract::{Contract, DocumentType};
use crate::drive::Drive;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::query::DriveQuery;

use dpp::data_contract::extra::DriveContractExt;

impl Drive {
    pub fn query_documents(
        &self,
        query_cbor: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let contract = self
            .get_contract(contract_id, transaction, &mut drive_operations)?
            .ok_or(Error::Query(QueryError::ContractNotFound(
                "contract not found",
            )))?;
        let document_type = contract.document_type_for_name(document_type_name)?;
        let (items, skipped) = self.query_documents_from_contract_internal(
            &contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, skipped, cost))
    }

    pub fn query_documents_from_contract_cbor(
        &self,
        contract_cbor: &[u8],
        document_type_name: String,
        query_cbor: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor, None)?;
        //todo cbor cost
        let document_type = contract.document_type_for_name(document_type_name.as_str())?;

        let (items, skipped) = self.query_documents_from_contract_internal(
            &contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, skipped, cost))
    }

    pub fn query_documents_from_contract(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let (items, skipped) = self.query_documents_from_contract_internal(
            &contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, skipped, cost))
    }

    pub(crate) fn query_documents_from_contract_internal(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let query = DriveQuery::from_cbor(query_cbor, contract, document_type)?;

        query.execute_no_proof_internal(self, transaction, drive_operations)
    }

    pub fn query_documents_as_grove_proof(
        &self,
        query_cbor: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        transaction: TransactionArg,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let contract = self
            .get_contract(contract_id, transaction, &mut drive_operations)?
            .ok_or(Error::Query(QueryError::ContractNotFound(
                "contract not found",
            )))?;
        let document_type = contract.document_type_for_name(document_type_name)?;
        let items = self.query_documents_from_contract_as_grove_proof_internal(
            &contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, cost))
    }

    pub fn query_documents_from_contract_cbor_as_grove_proof(
        &self,
        contract_cbor: &[u8],
        document_type_name: String,
        query_cbor: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor, None)?;

        let document_type = contract.document_type_for_name(document_type_name.as_str())?;

        let items = self.query_documents_from_contract_as_grove_proof_internal(
            &contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, cost))
    }

    pub fn query_documents_from_contract_as_grove_proof(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        let items = self.query_documents_from_contract_as_grove_proof_internal(
            contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, cost))
    }

    pub(crate) fn query_documents_from_contract_as_grove_proof_internal(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Vec<u8>, Error> {
        let query = DriveQuery::from_cbor(query_cbor, contract, document_type)?;

        query.execute_with_proof_internal(self, transaction, drive_operations)
    }

    pub fn query_documents_from_contract_as_grove_proof_only_get_elements(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        transaction: TransactionArg,
    ) -> Result<([u8; 32], Vec<Vec<u8>>, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        let (root_hash, items) = self
            .query_documents_from_contract_as_grove_proof_only_get_elements_internal(
                contract,
                document_type,
                query_cbor,
                transaction,
                &mut drive_operations,
            )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((root_hash, items, cost))
    }

    pub(crate) fn query_documents_from_contract_as_grove_proof_only_get_elements_internal(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<([u8; 32], Vec<Vec<u8>>), Error> {
        let query = DriveQuery::from_cbor(query_cbor, contract, document_type)?;

        query.execute_with_proof_only_get_elements_internal(self, transaction, drive_operations)
    }
}
