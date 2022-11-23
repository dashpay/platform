// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Drive Queries
//!
//! Defines and implements in Drive functions relevant to querying.
//!

use grovedb::TransactionArg;

use crate::contract::Contract;
use crate::drive::Drive;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::query::DriveQuery;
use dpp::data_contract::extra::DocumentType;

use crate::drive::block_info::BlockInfo;
use crate::fee_pools::epochs::Epoch;
use dpp::data_contract::extra::DriveContractExt;

impl Drive {
    /// Performs and returns the result of the specified query along with skipped items and the cost.
    pub fn query_documents(
        &self,
        query_cbor: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let contract = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                epoch,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Query(QueryError::ContractNotFound(
                "contract not found",
            )))?;
        let document_type = contract
            .contract
            .document_type_for_name(document_type_name)?;
        let (items, skipped) = self.query_documents_from_contract_internal(
            &contract.contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let cost = if let Some(epoch) = epoch {
            let fee_result = calculate_fee(None, Some(drive_operations), epoch)?;
            fee_result.processing_fee
        } else {
            0
        };

        Ok((items, skipped, cost))
    }

    /// Performs and returns the result of the specified query along with skipped items and the cost.
    pub fn query_documents_from_contract_cbor(
        &self,
        contract_cbor: &[u8],
        document_type_name: String,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
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
        let cost = if let Some(block_info) = block_info {
            let fee_result = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, skipped, cost))
    }

    /// Performs and returns the result of the specified query along with skipped items and the cost.
    pub fn query_documents_from_contract(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let (items, skipped) = self.query_documents_from_contract_internal(
            contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, skipped, cost))
    }

    /// Performs and returns the result of the specified query along with skipped items.
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

    /// Performs and returns the result of the specified query along with the fee.
    /// Proof is generated.
    pub fn query_documents_as_grove_proof(
        &self,
        query_cbor: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        block_info: Option<BlockInfo>,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let contract = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                epoch,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Query(QueryError::ContractNotFound(
                "contract not found",
            )))?;
        let document_type = contract
            .contract
            .document_type_for_name(document_type_name)?;
        let items = self.query_documents_from_contract_as_grove_proof_internal(
            &contract.contract,
            document_type,
            query_cbor,
            transaction,
            &mut drive_operations,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    /// Performs and returns the result of the specified query along with the fee.
    /// Proof is generated.
    pub fn query_documents_from_contract_cbor_as_grove_proof(
        &self,
        contract_cbor: &[u8],
        document_type_name: String,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
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
        let cost = if let Some(block_info) = block_info {
            let fee_result = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    /// Performs and returns the result of the specified query along with the fee.
    /// Proof is generated.
    pub fn query_documents_from_contract_as_grove_proof(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
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
        let cost = if let Some(block_info) = block_info {
            let fee_result = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    /// Performs and returns the result of the specified internal query.
    /// Proof is generated.
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

    /// Performs the specified internal query and returns the root hash, values, and fee.
    pub fn query_documents_from_contract_as_grove_proof_only_get_elements(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        query_cbor: &[u8],
        block_info: Option<BlockInfo>,
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
        let cost = if let Some(block_info) = block_info {
            let fee_result = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((root_hash, items, cost))
    }

    /// Performs the specified internal query and returns the root hash and values.
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
