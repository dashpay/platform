use dapi_grpc::platform::v0::StateTransitionBroadcastError;
use tonic::Status;

/// Map Drive/Tenderdash error codes to gRPC Status consistently
pub fn map_drive_code_to_status(code: u32, info: Option<String>) -> Status {
    let message = info.unwrap_or_else(|| format!("Drive error code: {}", code));
    match code {
        1 => Status::invalid_argument(message),
        2 => Status::failed_precondition(message),
        3 => Status::out_of_range(message),
        4 => Status::unimplemented(message),
        5 => Status::internal(message),
        6 => Status::unavailable(message),
        7 => Status::unauthenticated(message),
        8 => Status::permission_denied(message),
        9 => Status::aborted(message),
        10 => Status::out_of_range(message),
        11 => Status::unimplemented(message),
        12 => Status::internal(message),
        13 => Status::internal(message),
        14 => Status::unavailable(message),
        15 => Status::data_loss(message),
        16 => Status::unauthenticated(message),
        _ => Status::unknown(message),
    }
}

/// Build StateTransitionBroadcastError consistently from code/info/data
pub fn build_state_transition_error(
    code: u32,
    info: &str,
    data: Option<&str>,
) -> StateTransitionBroadcastError {
    let mut error = StateTransitionBroadcastError {
        code,
        message: info.to_string(),
        data: Vec::new(),
    };

    if let Some(data_str) = data {
        if let Ok(data_bytes) =
            base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, data_str)
        {
            error.data = data_bytes;
        }
    }

    error
}
