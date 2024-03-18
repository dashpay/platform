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

//! Tenderdash ABCI Messages.
//!
//! This module defines the structs used for handling ABCI messages
//! as well as defining and implementing the trait for serializing/deserializing them.
//!

use crate::abci::AbciError;
use dpp::util::deserializer::ProtocolVersion;
use drive::dpp::identity::TimestampMillis;
use serde::{Deserialize, Serialize};
use tenderdash_abci::proto::abci::RequestInitChain;
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

/// A struct for handling chain initialization requests
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestInitChainCleanedParams {
    /// The genesis time in milliseconds
    pub genesis_time: TimestampMillis,

    /// Initial height
    pub initial_height: u64,

    /// Initial core chain lock height.
    pub initial_core_height: Option<u32>,

    /// Initial protocol version
    pub initial_protocol_version: ProtocolVersion,
}

impl TryFrom<RequestInitChain> for RequestInitChainCleanedParams {
    type Error = AbciError;
    fn try_from(request: RequestInitChain) -> Result<Self, Self::Error> {
        let genesis_time = request
            .time
            .ok_or(AbciError::BadRequest(
                "genesis time is required in init chain".to_string(),
            ))?
            .to_milis() as TimestampMillis;
        let initial_core_height = match request.initial_core_height {
            0 => None,
            h => Some(h),
        };

        let consensus_params = request.consensus_params.ok_or(AbciError::BadRequest(
            "consensus params are required in init chain".to_string(),
        ))?;

        let tenderdash_abci::proto::types::VersionParams { app_version } =
            consensus_params.version.ok_or(AbciError::BadRequest(
                "consensus params version is required in init chain".to_string(),
            ))?;

        Ok(Self {
            genesis_time,
            initial_height: request.initial_height as u64,
            initial_core_height,
            initial_protocol_version: app_version as ProtocolVersion,
        })
    }
}
