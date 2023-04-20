// use crate::platform::Platform;
// use anyhow::{bail, Result as AnyResult};
// use dashcore_rpc::dashcore::anyhow::Result;
// use dashcore_rpc::dashcore::InstantLock;
// use dpp::async_trait::async_trait;
// use dpp::data_contract::{DataContract, DriveContractExt};
// use dpp::document::{Document, ExtendedDocument};
// use dpp::identifier::Identifier;
// use dpp::identity::{Identity, IdentityPublicKey, KeyID};
// use dpp::platform_value::Value;
// use dpp::prelude::{Revision, TimestampMillis};
// use dpp::state_repository::{FetchTransactionResponse, StateRepositoryLike};
// use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
// use drive::query::DriveQuery;
// use serde::Deserialize;
// use std::convert::Infallible;
//
// #[async_trait(?Send)]
// impl<'a, CoreRPCLike: Sync> StateRepositoryLike for Platform<'a, CoreRPCLike> {
//     type ConversionError = Infallible;
//     type FetchDataContract = DataContract;
//     type FetchDocument = Document;
//     type FetchExtendedDocument = ExtendedDocument;
//     type FetchIdentity = Identity;
//     type FetchTransaction = FetchTransactionResponse;
//
//     async fn fetch_data_contract(
//         &self,
//         data_contract_id: &Identifier,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<Option<Self::FetchDataContract>> {
//         let state = self.state.read().unwrap();
//         let block_execution_context = self.block_execution_context.read().unwrap().ok_or(anyhow::Error::new("there should be an execution context when calling fetch_data_contract from dpp via state repository"))?;
//         //todo: deal with fees
//         let (_, contract) = self.drive.get_contract_with_fetch_info(
//             data_contract_id.to_buffer(),
//             Some(&state.current_epoch),
//             Some(&block_execution_context.current_transaction),
//         )?;
//         Ok(contract.map(|c| c.contract.clone()))
//     }
//
//     async fn fetch_documents(
//         &self,
//         contract_id: &Identifier,
//         data_contract_type: &str,
//         where_query: Value,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<Vec<Self::FetchDocument>> {
//         let state = self.state.read().unwrap();
//         let block_execution_context = self.block_execution_context.read().unwrap().ok_or(anyhow::Error::new("there should be an execution context when calling fetch_data_contract from dpp via state repository"))?;
//         let (_, maybe_contract) = self.drive.get_contract_with_fetch_info(
//             contract_id.to_buffer(),
//             Some(&state.current_epoch),
//             Some(&block_execution_context.current_transaction),
//         )?;
//
//         let contract_fetch_info = maybe_contract.ok_or(anyhow::Error::new(
//             "the contract should exist when fetching documents",
//         ))?;
//         let contract = &contract_fetch_info.contract;
//         let document_type = contract
//             .document_type_for_name(data_contract_type)
//             .map_err(|_| {
//                 anyhow::Error::new(
//                     "the contract document type should exist when fetching documents",
//                 )
//             })?;
//         let drive_query = DriveQuery::from_value(where_query, contract, document_type)?;
//         //todo: deal with fees
//         let documents = self.drive.query_documents(
//             drive_query,
//             Some(&state.current_epoch),
//             Some(&block_execution_context.current_transaction),
//         )?;
//         Ok(documents.documents)
//     }
//
//     async fn fetch_extended_documents(
//         &self,
//         contract_id: &Identifier,
//         data_contract_type: &str,
//         where_query: Value,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<Vec<Self::FetchExtendedDocument>> {
//         let state = self.state.read().unwrap();
//         let block_execution_context = self.block_execution_context.read().unwrap().ok_or(anyhow::Error::new("there should be an execution context when calling fetch_data_contract from dpp via state repository"))?;
//         let (_, maybe_contract) = self.drive.get_contract_with_fetch_info(
//             contract_id.to_buffer(),
//             Some(&state.current_epoch),
//             Some(&block_execution_context.current_transaction),
//         )?;
//
//         let contract_fetch_info = maybe_contract.ok_or(anyhow::Error::new(
//             "the contract should exist when fetching documents",
//         ))?;
//         let contract = &contract_fetch_info.contract;
//         let document_type = contract
//             .document_type_for_name(data_contract_type)
//             .map_err(|_| {
//                 anyhow::Error::new(
//                     "the contract document type should exist when fetching documents",
//                 )
//             })?;
//         let drive_query = DriveQuery::from_value(where_query, contract, document_type)?;
//         //todo: deal with fees
//         let documents = self.drive.query_documents(
//             drive_query,
//             Some(&state.current_epoch),
//             Some(&block_execution_context.current_transaction),
//         )?;
//         let extended_documents = documents
//             .documents
//             .into_iter()
//             .map(|document| {
//                 ExtendedDocument::from_document_with_additional_info(
//                     document,
//                     contract.clone(),
//                     data_contract_type.to_string(),
//                     state.current_protocol_version_in_consensus,
//                 )
//             })
//             .collect();
//         Ok(extended_documents)
//     }
//
//     async fn fetch_transaction(
//         &self,
//         id: &str,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<Self::FetchTransaction> {
//         todo!()
//     }
//
//     async fn fetch_identity(
//         &self,
//         id: &Identifier,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<Option<Self::FetchIdentity>> {
//         todo!()
//     }
//
//     async fn fetch_identity_balance(
//         &self,
//         identity_id: &Identifier,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<Option<u64>> {
//         todo!()
//     }
//
//     async fn fetch_identity_balance_with_debt(
//         &self,
//         identity_id: &Identifier,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<Option<i64>> {
//         todo!()
//     }
//
//     async fn fetch_latest_platform_block_header(&self) -> AnyResult<Vec<u8>> {
//         todo!()
//     }
//
//     async fn verify_instant_lock(
//         &self,
//         instant_lock: &InstantLock,
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<bool> {
//         todo!()
//     }
//
//     async fn is_asset_lock_transaction_out_point_already_used(
//         &self,
//         out_point_buffer: &[u8],
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<bool> {
//         todo!()
//     }
//
//     async fn mark_asset_lock_transaction_out_point_as_used(
//         &self,
//         out_point_buffer: &[u8],
//         execution_context: Option<&'a StateTransitionExecutionContext>,
//     ) -> AnyResult<()> {
//         todo!()
//     }
//
//     async fn fetch_sml_store<T>(&self) -> AnyResult<T>
//     where
//         T: for<'de> Deserialize<'de> + 'static,
//     {
//         todo!()
//     }
//
//     async fn fetch_latest_withdrawal_transaction_index(&self) -> AnyResult<u64> {
//         todo!()
//     }
//
//     async fn fetch_latest_platform_core_chain_locked_height(&self) -> AnyResult<Option<u32>> {
//         todo!()
//     }
//
//     async fn enqueue_withdrawal_transaction(
//         &self,
//         index: u64,
//         transaction_bytes: Vec<u8>,
//     ) -> AnyResult<()> {
//         todo!()
//     }
//
//     async fn fetch_latest_platform_block_time(&self) -> AnyResult<u64> {
//         todo!()
//     }
//
//     async fn fetch_latest_platform_block_height(&self) -> AnyResult<u64> {
//         todo!()
//     }
// }
