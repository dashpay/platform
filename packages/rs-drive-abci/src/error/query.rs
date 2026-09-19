use dpp::platform_value::Error as ValueError;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::FeatureVersion;
use dpp::ProtocolError;
use drive::error::proof::ProofError;
use drive::error::query::QuerySyntaxError as SyntaxError;
use drive::error::Error as DriveError;
use platform_query_wire::proto_conversions::DecodeError as WireDecodeError;
use prost::DecodeError;
use tenderdash_abci::proto::abci::ResponseException;

// @append_only
/// Errors
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    /// Proof Error
    #[error("proof error: {0}")]
    Proof(#[from] ProofError),

    /// Syntax Error
    #[error("query syntax error: {0}")]
    Query(#[from] SyntaxError),

    /// Protocol Error
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Value Error
    #[error("query value error: {0}")]
    Value(#[from] ValueError),

    /// Drive Error
    #[error("drive error: {0}")]
    Drive(#[from] DriveError),

    /// Decoding error Error
    #[error("protobuf decoding error: {0}")]
    ProtobufDecode(#[from] DecodeError),

    /// Invalid argument Error
    #[error("invalid argument error: {0}")]
    InvalidArgument(String),

    /// Too many elements Error
    #[error("too many elements error: {0}")]
    TooManyElements(String),

    /// Not found Error
    #[error("not found error: {0}")]
    NotFound(String),

    /// Server issue
    #[error("query not serviceable: {0}")]
    NotServiceable(String),

    /// Decoding Error
    #[error("decoding error: {0}")]
    DecodingError(String),

    /// Not found Error
    #[error("unsupported version for query: {0}, currently supporting versions {1} to {2} on platform protocol {3}, given {4}")]
    UnsupportedQueryVersion(
        String,
        FeatureVersion,
        FeatureVersion,
        ProtocolVersion,
        FeatureVersion,
    ),

    /// Server-side query capacity was exhausted.
    #[error("query resource exhausted: {0}")]
    ResourceExhausted(String),
}

/// Wire-decode failures from the shared `platform-query-wire` decoders.
/// `InvalidArgument` is malformed wire input; `Unsupported` is a well-formed
/// shape the decoder deliberately refuses (e.g. `ORDER BY` on aggregate
/// keys) and surfaces as `QuerySyntaxError::Unsupported`, the same variant
/// the v1 handler's `not_yet_implemented` path uses. Both carry the message
/// string through unchanged.
impl From<WireDecodeError> for QueryError {
    fn from(error: WireDecodeError) -> Self {
        match error {
            WireDecodeError::InvalidArgument(msg) => QueryError::InvalidArgument(msg),
            WireDecodeError::Unsupported(msg) => QueryError::Query(SyntaxError::Unsupported(msg)),
        }
    }
}

impl From<QueryError> for ResponseException {
    fn from(value: QueryError) -> Self {
        Self {
            error: value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared decoder's two variants map onto distinct `QueryError`
    /// surfaces and the message text is preserved verbatim. A decoder that
    /// later reclassifies a malformed shape as `Unsupported` (or vice versa)
    /// changes what clients see, so the mapping is pinned here.
    #[test]
    fn wire_decode_error_mapping_preserves_variant_and_message() {
        let invalid: QueryError = WireDecodeError::InvalidArgument("bad where".to_string()).into();
        assert!(
            matches!(&invalid, QueryError::InvalidArgument(msg) if msg == "bad where"),
            "unexpected: {invalid:?}"
        );

        let unsupported: QueryError = WireDecodeError::Unsupported(
            "ORDER BY on aggregate keys is not yet implemented".to_string(),
        )
        .into();
        assert!(
            matches!(
                &unsupported,
                QueryError::Query(SyntaxError::Unsupported(msg))
                    if msg == "ORDER BY on aggregate keys is not yet implemented"
            ),
            "unexpected: {unsupported:?}"
        );
    }
}
