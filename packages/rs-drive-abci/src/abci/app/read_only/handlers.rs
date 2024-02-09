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

//! Tenderdash ABCI Handlers.
//!
//! This module defines the `TenderdashAbci` trait and implements it for type `Platform`.
//!
//! Handlers in this function MUST be version agnostic, meaning that for all future versions, we
//! can only make changes that are backwards compatible. Otherwise new calls must be made instead.
//!

use crate::abci::app::read_only::ReadOnlyAbciApplication;
use crate::abci::handler::error::consensus::AbciResponseInfoGetter;
use crate::abci::handler::error::HandlerError;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::SignedCredits;
use dpp::version::PlatformVersion;
use dpp::version::PlatformVersionCurrentVersion;
use tenderdash_abci::proto::abci as proto;

impl<'a, C> tenderdash_abci::Application for ReadOnlyAbciApplication<'a, C>
where
    C: CoreRPCLike,
{
    fn info(
        &self,
        _request: proto::RequestInfo,
    ) -> Result<proto::ResponseInfo, proto::ResponseException> {
        unreachable!("info is not implemented for read-only ABCI application")
    }

    fn init_chain(
        &self,
        _request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, proto::ResponseException> {
        unreachable!("init_chain is not implemented for read-only ABCI application")
    }

    fn query(
        &self,
        request: proto::RequestQuery,
    ) -> Result<proto::ResponseQuery, proto::ResponseException> {
        let _timer = crate::metrics::abci_request_duration("query");

        let proto::RequestQuery { data, path, .. } = &request;

        // TODO: It must be proto::ResponseException
        let Some(platform_version) = PlatformVersion::get_maybe_current() else {
            let handler_error =
                HandlerError::Unavailable("platform is not initialized".to_string());

            let response = proto::ResponseQuery {
                code: handler_error.code(),
                log: "".to_string(),
                info: handler_error.response_info()?,
                index: 0,
                key: vec![],
                value: vec![],
                proof_ops: None,
                height: self.platform.state.read().unwrap().last_committed_height() as i64,
                codespace: "".to_string(),
            };

            tracing::error!(?response, "platform version not initialized");

            return Ok(response);
        };

        let result = self
            .platform
            .query(path.as_str(), data.as_slice(), platform_version)?;

        let (code, data, info) = if result.is_valid() {
            (0, result.data.unwrap_or_default(), "success".to_string())
        } else {
            let error = result
                .errors
                .first()
                .expect("validation result should have at least one error");

            let handler_error = HandlerError::from(error);

            (handler_error.code(), vec![], handler_error.response_info()?)
        };

        let response = proto::ResponseQuery {
            //todo: right now just put GRPC error codes,
            //  later we will use own error codes
            code,
            log: "".to_string(),
            info,
            index: 0,
            key: vec![],
            value: data,
            proof_ops: None,
            height: self.platform.state.read().unwrap().last_committed_height() as i64,
            codespace: "".to_string(),
        };

        Ok(response)
    }

    fn check_tx(
        &self,
        request: proto::RequestCheckTx,
    ) -> Result<proto::ResponseCheckTx, proto::ResponseException> {
        let _timer = crate::metrics::abci_request_duration("check_tx");

        let proto::RequestCheckTx { tx, r#type } = request;
        match self.platform.check_tx(tx.as_slice(), r#type.try_into()?) {
            Ok(validation_result) => {
                let platform_state = self.platform.state.read().unwrap();
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

    fn extend_vote(
        &self,
        _request: proto::RequestExtendVote,
    ) -> Result<proto::ResponseExtendVote, proto::ResponseException> {
        unreachable!("extend_vote is not implemented for read-only ABCI application")
    }

    fn finalize_block(
        &self,
        _request: proto::RequestFinalizeBlock,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException> {
        // TODO: We don't need this

        self.platform
            .drive
            .grove
            .try_to_catch_up_from_primary()
            .expect("failed to catch up");

        Ok(proto::ResponseFinalizeBlock {
            events: vec![],
            retain_height: 0,
        })
    }

    fn prepare_proposal(
        &self,
        _request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, proto::ResponseException> {
        unreachable!("prepare_proposal is not implemented for read-only ABCI application")
    }

    fn process_proposal(
        &self,
        _request: proto::RequestProcessProposal,
    ) -> Result<proto::ResponseProcessProposal, proto::ResponseException> {
        unreachable!("process_proposal is not implemented for read-only ABCI application")
    }

    fn verify_vote_extension(
        &self,
        _request: proto::RequestVerifyVoteExtension,
    ) -> Result<proto::ResponseVerifyVoteExtension, proto::ResponseException> {
        unreachable!("verify_vote_extension is not implemented for read-only ABCI application")
    }
}
