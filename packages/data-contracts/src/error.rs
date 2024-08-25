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

impl From<withdrawals_contract::Error> for Error {
    fn from(e: withdrawals_contract::Error) -> Self {
        match e {
            withdrawals_contract::Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            } => Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            },
            withdrawals_contract::Error::InvalidSchemaJson(e) => Error::InvalidSchemaJson(e),
        }
    }
}

impl From<dashpay_contract::Error> for Error {
    fn from(e: dashpay_contract::Error) -> Self {
        match e {
            dashpay_contract::Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            } => Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            },
            dashpay_contract::Error::InvalidSchemaJson(e) => Error::InvalidSchemaJson(e),
        }
    }
}

impl From<dpns_contract::Error> for Error {
    fn from(e: dpns_contract::Error) -> Self {
        match e {
            dpns_contract::Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            } => Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            },
            dpns_contract::Error::InvalidSchemaJson(e) => Error::InvalidSchemaJson(e),
        }
    }
}

impl From<masternode_reward_shares_contract::Error> for Error {
    fn from(e: masternode_reward_shares_contract::Error) -> Self {
        match e {
            masternode_reward_shares_contract::Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            } => Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            },
            masternode_reward_shares_contract::Error::InvalidSchemaJson(e) => {
                Error::InvalidSchemaJson(e)
            }
        }
    }
}

impl From<feature_flags_contract::Error> for Error {
    fn from(e: feature_flags_contract::Error) -> Self {
        match e {
            feature_flags_contract::Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            } => Error::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            },
            feature_flags_contract::Error::InvalidSchemaJson(e) => Error::InvalidSchemaJson(e),
        }
    }
}
