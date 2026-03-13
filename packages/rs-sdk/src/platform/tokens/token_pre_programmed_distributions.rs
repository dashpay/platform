use crate::platform::{Fetch, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::get_token_pre_programmed_distributions_request_v0::StartAtInfo;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::GetTokenPreProgrammedDistributionsRequestV0;
use dapi_grpc::platform::v0::{
    get_token_pre_programmed_distributions_request, GetTokenPreProgrammedDistributionsRequest,
};
use dpp::prelude::Identifier;
pub use drive_proof_verifier::types::TokenPreProgrammedDistributions;

/// Query parameters for fetching pre-programmed token distributions.
#[derive(Debug, Clone)]
pub struct TokenPreProgrammedDistributionsQuery {
    /// Token identifier.
    pub token_id: Identifier,
    /// Optional pagination start point. When `None`, results start from the earliest timestamp.
    pub start_at_info: Option<TokenPreProgrammedDistributionsStartAtInfo>,
    /// Optional limit on the number of results.
    pub limit: Option<u32>,
}

/// Start-at pagination info for pre-programmed distributions queries.
#[derive(Debug, Clone)]
pub struct TokenPreProgrammedDistributionsStartAtInfo {
    /// Timestamp in milliseconds to start from.
    pub start_time_ms: u64,
    /// Optional recipient identifier to start from within the timestamp.
    pub start_recipient: Option<Identifier>,
    /// Whether to include the start recipient in results. Defaults to `true` when absent in proof
    /// verification, so omitting this is equivalent to an inclusive range start.
    pub start_recipient_included: bool,
}

impl Query<GetTokenPreProgrammedDistributionsRequest> for TokenPreProgrammedDistributionsQuery {
    fn query(self, prove: bool) -> Result<GetTokenPreProgrammedDistributionsRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let start_at_info = self.start_at_info.map(|info| {
            let has_recipient = info.start_recipient.is_some();
            StartAtInfo {
                start_time_ms: info.start_time_ms,
                start_recipient: info.start_recipient.map(|id| id.to_vec()),
                // Only set when a start_recipient is provided; meaningless otherwise.
                start_recipient_included: if has_recipient {
                    Some(info.start_recipient_included)
                } else {
                    None
                },
            }
        });

        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(get_token_pre_programmed_distributions_request::Version::V0(
                GetTokenPreProgrammedDistributionsRequestV0 {
                    token_id: self.token_id.to_vec(),
                    start_at_info,
                    limit: self.limit,
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Fetch for TokenPreProgrammedDistributions {
    type Request = GetTokenPreProgrammedDistributionsRequest;
}
