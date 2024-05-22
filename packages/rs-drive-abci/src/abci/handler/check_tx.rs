use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::abci::handler::error::HandlerError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::metrics::{LABEL_ABCI_RESPONSE_CODE, LABEL_CHECK_TX_MODE, LABEL_STATE_TRANSITION_NAME};
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::SignedCredits;
use dpp::util::hash::hash_single;
use metrics::Label;
use tenderdash_abci::proto::abci as proto;

pub fn check_tx<C>(
    platform: &Platform<C>,
    request: proto::RequestCheckTx,
) -> Result<proto::ResponseCheckTx, Error>
where
    C: CoreRPCLike,
{
    let mut timer = crate::metrics::abci_request_duration("check_tx");

    let platform_state = platform.state.load();
    let platform_version = platform_state.current_platform_version()?;

    let proto::RequestCheckTx { tx, r#type } = request;

    let validation_result = platform.check_tx(
        tx.as_slice(),
        r#type.try_into()?,
        &platform_state,
        platform_version,
    );

    validation_result
        .and_then(|validation_result| {
            let (check_tx_result, errors) =
                validation_result.into_data_and_errors().map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "validation result should contain check tx result",
                    ))
                })?;

            let first_consensus_error = errors.first();

            let (code, info) = if let Some(consensus_error) = first_consensus_error {
                (
                    consensus_error.code(),
                    consensus_error.response_info_for_version(platform_version)?,
                )
            } else {
                // If there are no execution errors the code will be 0
                (0, "".to_string())
            };

            let gas_wanted = check_tx_result
                .fee_result
                .as_ref()
                .map(|fee_result| fee_result.total_base_fee())
                .unwrap_or_default();

            // Todo: IMPORTANT We need tenderdash to support multiple senders
            let first_unique_identifier = check_tx_result
                .unique_identifiers
                .first()
                .cloned()
                .unwrap_or_default();

            let state_transition_name = check_tx_result
                .state_transition_name
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());

            let priority = check_tx_result.priority as i64;

            if tracing::enabled!(tracing::Level::TRACE) {
                let message = match (r#type, code) {
                    (0, 0) => "added to mempool".to_string(),
                    (1, 0) => "kept in mempool after re-check".to_string(),
                    (0, _) => format!(
                        "rejected with code {code} due to error: {}",
                        first_consensus_error.ok_or_else(|| Error::Execution(
                            ExecutionError::CorruptedCodeExecution(
                                "consensus error must be present with non-zero error code"
                            )
                        ))?
                    ),
                    (1, _) => format!(
                        "removed from mempool with code {code} after re-check due to error: {}",
                        first_consensus_error.ok_or_else(|| Error::Execution(
                            ExecutionError::CorruptedCodeExecution(
                                "consensus error must be present with non-zero error code"
                            )
                        ))?
                    ),
                    _ => {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "we have only 2 modes of check tx",
                        )))
                    }
                };

                let state_transition_hash =
                    check_tx_result.state_transition_hash.ok_or_else(|| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "state transition hash must be present if trace level is enabled",
                        ))
                    })?;

                let st_hash = hex::encode(state_transition_hash);

                tracing::trace!(
                    ?check_tx_result,
                    error = ?first_consensus_error,
                    st_hash,
                    "{} state transition ({}) {}",
                    state_transition_name,
                    st_hash,
                    message
                );
            }

            timer.add_label(Label::new(
                LABEL_STATE_TRANSITION_NAME,
                state_transition_name,
            ));
            timer.add_label(Label::new(LABEL_CHECK_TX_MODE, r#type.to_string()));
            timer.add_label(Label::new(LABEL_ABCI_RESPONSE_CODE, code.to_string()));

            Ok(proto::ResponseCheckTx {
                code,
                data: vec![],
                info,
                gas_wanted: gas_wanted as SignedCredits,
                codespace: "".to_string(),
                sender: first_unique_identifier,
                priority,
            })
        })
        .or_else(|error| {
            let handler_error = HandlerError::Internal(error.to_string());

            if tracing::enabled!(tracing::Level::ERROR) {
                let st_hash = hex::encode(hash_single(tx));

                tracing::error!(
                    ?error,
                    st_hash,
                    check_tx_mode = r#type,
                    "Failed to check state transition ({}): {}",
                    st_hash,
                    error
                );
            }

            Ok(proto::ResponseCheckTx {
                code: handler_error.code(),
                data: vec![],
                info: handler_error.response_info()?,
                gas_wanted: 0 as SignedCredits,
                codespace: "".to_string(),
                sender: "".to_string(),
                priority: 0,
            })
        })
}
