use platform_version::version::FeatureVersion;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Platform expected some specific versions
    #[error("platform unknown version on {method}, received: {received}")]
    UnknownVersionMismatch {
        /// method
        method: String,
        /// the allowed versions for this method
        known_versions: Vec<FeatureVersion>,
        /// requested core height
        received: FeatureVersion,
    },
    #[error("schema deserialize error: {0}")]
    InvalidSchemaJson(#[from] serde_json::Error),
}
