//! `GetConsensusParams` request.

use dapi_grpc::platform::v0::{self as platform_proto};

use crate::{transport::TransportRequest, DapiRequest, Settings};

use super::IncompleteMessage;

/// Request consensus params.
#[derive(Debug)]
pub struct GetConsensusParams {
    /// Block height
    pub height: u32,
}

/// [GetConsensusParams] response.
#[derive(Debug)]
pub struct GetConsensusParamsResponse {
    /// TODO
    pub block: ConsensusParamsBlock,
    /// TODO
    pub evidence: ConsensusParamsEvidence,
}

/// TODO
#[derive(Debug)]
pub struct ConsensusParamsBlock {
    /// TODO
    pub max_bytes: String,
    /// TODO
    pub max_gas: String,
    /// TODO
    pub time_iota_ms: String,
}

impl From<platform_proto::ConsensusParamsBlock> for ConsensusParamsBlock {
    fn from(proto: platform_proto::ConsensusParamsBlock) -> Self {
        ConsensusParamsBlock {
            max_bytes: proto.max_bytes,
            max_gas: proto.max_gas,
            time_iota_ms: proto.time_iota_ms,
        }
    }
}

/// TODO
#[derive(Debug)]
pub struct ConsensusParamsEvidence {
    /// TODO
    pub max_age_num_blocks: String,
    /// TODO
    pub max_age_duration: String,
    /// TODO
    pub max_bytes: String,
}

impl From<platform_proto::ConsensusParamsEvidence> for ConsensusParamsEvidence {
    fn from(proto: platform_proto::ConsensusParamsEvidence) -> Self {
        ConsensusParamsEvidence {
            max_age_num_blocks: proto.max_age_num_blocks,
            max_age_duration: proto.max_age_duration,
            max_bytes: proto.max_bytes,
        }
    }
}

impl DapiRequest for GetConsensusParams {
    type DapiResponse = GetConsensusParamsResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetConsensusParamsRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetConsensusParamsRequest {
            height: self.height as i64,
            prove: false,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        if let platform_proto::GetConsensusParamsResponse {
            block: Some(block),
            evidence: Some(evidence),
        } = transport_response
        {
            Ok(GetConsensusParamsResponse {
                block: block.into(),
                evidence: evidence.into(),
            })
        } else {
            Err(IncompleteMessage)
        }
    }
}
