use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::abci::handler::error::HandlerError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::SignedCredits;
use tenderdash_abci::proto::abci as proto;

pub fn check_tx<C>(
    platform: &Platform<C>,
    request: proto::RequestCheckTx,
) -> Result<proto::ResponseCheckTx, Error>
where
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("check_tx");

    let platform_state = platform.state.read();
    let platform_version = platform_state.current_platform_version()?;
    drop(platform_state);

    let proto::RequestCheckTx { tx, r#type } = request;
    match platform.check_tx(tx.as_slice(), r#type.try_into()?, platform_version) {
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

            let check_tx_result = validation_result.data.unwrap_or_default();

            let gas_wanted = check_tx_result
                .fee_result
                .map(|fee_result| fee_result.total_base_fee())
                .unwrap_or_default();

            // Todo: IMPORTANT We need tenderdash to support multiple senders
            let first_unique_identifier = check_tx_result
                .unique_identifiers
                .first()
                .cloned()
                .unwrap_or_default();

            Ok(proto::ResponseCheckTx {
                code,
                data: vec![],
                info,
                gas_wanted: gas_wanted as SignedCredits,
                codespace: "".to_string(),
                sender: first_unique_identifier,
                priority: 0,
            })
        }
        Err(error) => {
            let handler_error = HandlerError::Internal(error.to_string());

            tracing::error!(?error, "check_tx failed");

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
