use dapi_grpc::tonic::Status;
use drive_abci::error::query::QueryError;
use drive_abci::error::Error;

pub fn query_error_into_status(error: QueryError) -> Status {
    match error {
        QueryError::NotFound(message) => Status::not_found(message),
        QueryError::InvalidArgument(message) => Status::invalid_argument(message),
        QueryError::Query(error) => Status::invalid_argument(error.to_string()),
        _ => Status::unknown(error.to_string()),
    }
}

pub fn error_into_status(error: Error) -> Status {
    Status::internal(format!("query: {}", error))
}
