use crate::abci::app::PlatformApplication;
use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::abci::handler::error::HandlerError;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::SignedCredits;
use tenderdash_abci::proto::abci as proto;

pub fn check_tx<A, C>(
    app: &A,
    request: proto::RequestCheckTx,
) -> Result<proto::ResponseCheckTx, proto::ResponseException>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("check_tx");

    let proto::RequestCheckTx { tx, r#type } = request;
    match app.platform().check_tx(tx.as_slice(), r#type.try_into()?) {
        Ok(validation_result) => {
            let platform_state = app.platform().state.read().unwrap();
            let platform_version = platform_state.current_platform_version()?;
            let first_consensus_error = validation_result.errors.first();

            let (code, info) = if let Some(consensus_error) = first_consensus_error {
                (
                    consensus_error.code(),
                    consensus_error
                        .response_info_for_version(platform_version)
                        .map_err(proto::ResponseException::from)?,
                )
            } else {
                // If there are no execution errors the code will be 0
                (0, "".to_string())
            };

            let gas_wanted = validation_result
                .data
                .map(|fee_result| {
                    fee_result
                        .map(|fee_result| fee_result.total_base_fee())
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            Ok(proto::ResponseCheckTx {
                code,
                data: vec![],
                info,
                gas_wanted: gas_wanted as SignedCredits,
                codespace: "".to_string(),
                sender: "".to_string(),
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
