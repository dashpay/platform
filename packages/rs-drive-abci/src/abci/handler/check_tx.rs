use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::abci::handler::error::HandlerError;
use crate::error::Error;
use crate::metrics::{LABEL_CHECK_TX_MODE, LABEL_CHECK_TX_RESPONSE, LABEL_STATE_TRANSITION_NAME};
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::SignedCredits;
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
    match platform.check_tx(
        tx.as_slice(),
        r#type.try_into()?,
        &platform_state,
        platform_version,
    ) {
        Ok(validation_result) => {
            let first_consensus_error = validation_result.errors.first();

            let (code, info) = if let Some(consensus_error) = first_consensus_error {
                (
                    consensus_error.code(),
                    consensus_error.response_info_for_version(platform_version)?,
                )
            } else {
                // If there are no execution errors the code will be 0
                (0, "".to_string())
            };

            // TODO: We shouldn't use default check tx result. It provides wrong information
            let check_tx_result = validation_result.data.unwrap_or_default();

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

            let state_transition_name =
                if let Some(ref name) = check_tx_result.state_transition_name {
                    name.to_owned()
                } else {
                    "Unknown".to_string()
                };

            let priority = check_tx_result.priority as i64;

            let message = match (r#type, code) {
                (0, 0) => "added to mempool".to_string(),
                (1, 0) => "kept in mempool after re-check".to_string(),
                (0, _) => format!("rejected with code {code}"),
                (1, _) => format!("removed from mempool with code {code} after re-check"),
                _ => unreachable!("we have only 2 modes of check tx"),
            };

            let state_transition_hash = check_tx_result
                .state_transition_hash
                .expect("state transition hash must be present");

            tracing::trace!(
                ?check_tx_result,
                "{} state transition {} {message}",
                state_transition_name,
                hex::encode(state_transition_hash),
            );

            timer.add_label(Label::new(
                LABEL_STATE_TRANSITION_NAME,
                state_transition_name,
            ));
            timer.add_label(Label::new(LABEL_CHECK_TX_MODE, r#type.to_string()));
            timer.add_label(Label::new(LABEL_CHECK_TX_RESPONSE, code.to_string()));

            Ok(proto::ResponseCheckTx {
                code,
                data: vec![],
                info,
                gas_wanted: gas_wanted as SignedCredits,
                codespace: "".to_string(),
                sender: first_unique_identifier,
                priority,
            })
        }
        Err(error) => {
            let handler_error = HandlerError::Internal(error.to_string());

            tracing::error!(?error, check_tx_mode = r#type, "check_tx failed: {}", error);

            timer.cancel();

            Ok(proto::ResponseCheckTx {
                code: handler_error.code(),
                data: vec![],
                info: handler_error.response_info()?,
                gas_wanted: 0 as SignedCredits,
                codespace: "".to_string(),
                sender: "".to_string(),
                priority: 0,
            })
        }
    }
}
