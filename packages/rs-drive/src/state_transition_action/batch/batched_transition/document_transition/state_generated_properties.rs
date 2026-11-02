//! The `generatedFrom` properties whose function reads state (`sys.tokens`, protocol version
//! 15), written or checked while a document create or replace action is built.

use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::{DocumentTypeRef, TokenFunction};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Value;
use dpp::tokens::calculate_token_id;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Writes or checks every property of `data` that `document_type` generates with a function
/// reading state (`DocumentTypeBasicMethods::generate_state_read_properties`), reading each
/// token it names: its contract info, then its contract, unless that is the document's own
/// (`contract`). Returns the refusal of the first property that does not pass, and the fee of
/// every read, which the write pays whether it passes or not. `stored` is the replaced
/// document's properties, `None` on a create.
///
/// Added in place at protocol version 15 and inert before it, where no type declares such a
/// function: then nothing is read and the fee is zero.
#[allow(clippy::too_many_arguments)]
pub(super) fn generate_state_read_properties(
    drive: &Drive,
    document_type: DocumentTypeRef,
    contract: &Arc<DataContractFetchInfo>,
    data: &mut BTreeMap<String, Value>,
    stored: Option<&BTreeMap<String, Value>>,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<(Option<ConsensusError>, FeeResult), Error> {
    if !document_type
        .generated_from_fields()
        .iter()
        .any(|(_, generated_from)| generated_from.function.reads_state())
    {
        return Ok((None, FeeResult::default()));
    }
    let mut fee = FeeResult::default();
    let refusal = document_type.generate_state_read_properties(
        data,
        stored,
        |function, token_id| -> Result<Option<Value>, Error> {
            let (token_contract_info, info_fee) = drive.fetch_token_contract_info_with_costs(
                token_id.to_buffer(),
                block_info,
                true,
                transaction,
                platform_version,
            )?;
            fee.checked_add_assign(info_fee)?;
            let Some(token_contract_info) = token_contract_info else {
                return Ok(None);
            };
            let token_contract_id = token_contract_info.contract_id();
            let position = token_contract_info.token_contract_position();
            if calculate_token_id(token_contract_id.as_bytes(), position) != token_id.to_buffer() {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract info does not match token {token_id}"
                ))));
            }
            let token_contract = if token_contract_id == contract.contract.id() {
                contract.clone()
            } else {
                let (contract_fee, token_contract) = drive.get_contract_with_fetch_info_and_fee(
                    token_contract_id.to_buffer(),
                    Some(&block_info.epoch),
                    false,
                    transaction,
                    platform_version,
                )?;
                fee.checked_add_assign(contract_fee.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution(
                        "a contract fetched with an epoch has a fee",
                    ),
                ))?)?;
                token_contract.ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "contract {token_contract_id} of token {token_id} is missing"
                    )))
                })?
            };
            let token_configuration =
                token_contract
                    .contract
                    .tokens()
                    .get(&position)
                    .ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "contract {token_contract_id} has no token at position {position}"
                        )))
                    })?;
            Ok(Some(match function {
                TokenFunction::Decimals => Value::U8(token_configuration.conventions().decimals()),
            }))
        },
    )?;
    Ok((refusal, fee))
}
