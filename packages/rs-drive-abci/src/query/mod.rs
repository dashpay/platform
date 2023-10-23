mod data_contract_based_queries;
mod document_query;
mod identity_based_queries;
mod proofs;
mod response_metadata;
mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::error::execution::ExecutionError;
use dpp::validation::ValidationResult;
use dpp::version::FeatureVersion;
use dpp::version::PlatformVersion;

#[macro_export]
macro_rules! generate_version_match_arms {
    ($max_version:expr, $current_version:expr) => {
        paste::paste! {
            Version::[< V $current_version >](_) => $current_version
        }
        ,
        generate_version_match_arms!($max_version, $current_version + 1)
    };
    ($max:expr, $current:expr) => {
        paste::paste! {
            Version::[< V $current >](_) => $current
        }
    };
}

#[macro_export]
macro_rules! generate_match_arms {
    ($type_name:ident, $feature_subfield:ident, 0) => {
        paste::paste! {
            $type_name::[< V 0 >](request) => {
                self.paste::paste!([< query_ $feature_subfield _v 0 >])(state, request, platform_version)
            }
        }
    };
    ($type_name:ident, $feature_subfield:ident, $current_version:expr) => {
        paste::paste! {
            $type_name::[< V $current_version >](request) => {
                self.paste::paste!([< query_ $feature_subfield _v $current_version >])(state, request, platform_version)
            },
        }
        generate_match_arms!($type_name, $feature_subfield, $current_version - 1)
    };
}

#[macro_export]
macro_rules! query_impl {
    ($type_name:ident, $feature_field:ident, $feature_subfield:ident, $max_known_version:expr) => {
        use dpp::check_validation_result_with_data;
        use dpp::validation::ValidationResult;
        use crate::error::query::QueryError;
        use crate::error::Error;
        use crate::platform_types::platform::Platform;
        use crate::platform_types::platform_state::PlatformState;
        use crate::query::QueryValidationResult;
        use dpp::version::PlatformVersion;
        use prost::Message;
        paste! {
            impl<C> Platform<C> {
                /// Querying
                pub(in crate::query) fn [<query_ $feature_subfield>](
                    &self,
                    state: &PlatformState,
                    query_data: &[u8],
                    platform_version: &PlatformVersion,
                ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
                    use dapi_grpc::platform::v0::paste::paste!([$type_name, "::Version"]);
                    use dpp::check_validation_result_with_data;

                    let $type_name { version } =
                        check_validation_result_with_data!($type_name::decode(query_data));

                    let Some(version) = version else {
                        return Ok(QueryValidationResult::new_with_error(
                            QueryError::DecodingError(concat!("could not decode ", stringify!($type_name)).to_string()),
                        ));
                    };

                    let feature_version_bounds = &platform_version
                        .drive_abci
                        .query
                        .$feature_field
                        .$feature_subfield;

                    let feature_version = match &version {
                        generate_version_match_arms!($max_known_version, 0)
                        _ => return Err(ExecutionError::UnknownVersionMismatch {
                            method: concat!("query_", stringify!($feature_subfield)).to_string(),
                            known_versions: (0..=$max_known_version).collect::<Vec<_>>(),
                            received: feature_version,
                        }.into()),
                    };

                    if !feature_version_bounds.check_version(feature_version) {
                        return Ok(QueryValidationResult::new_with_error(
                            QueryError::UnsupportedQueryVersion(
                                stringify!($feature_subfield).to_string(),
                                feature_version_bounds.min_version,
                                feature_version_bounds.max_version,
                                platform_version.protocol_version,
                                feature_version,
                            ),
                        ));
                    }

                    match version {
                        generate_match_arms!($type_name, $feature_subfield, $max_known_version)
                    }
                }
            }
        }
    };
}

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

impl<C> Platform<C> {
    /// Querying
    pub fn query(
        &self,
        query_path: &str,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        match platform_version.drive_abci.query.base_query_structure {
            0 => self.query_v0(query_path, query_data, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "Platform::query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
