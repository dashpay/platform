mod anchors;
mod encrypted_notes;
mod most_recent_anchor;
mod notes_count;
mod nullifiers;
mod pool_state;

use crate::error::query::QueryError;
use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_anchors_path_vec, shielded_credit_pool_nullifiers_path_vec,
    shielded_credit_pool_path_vec, shielded_latest_recorded_anchor_path_query,
    token_shielded_pool_anchors_path_vec, token_shielded_pool_latest_recorded_anchor_path_query,
    token_shielded_pool_nullifiers_path_vec, token_shielded_pool_path_vec,
};
use drive::grovedb::PathQuery;

/// Which shielded pool a shielded query targets: the credit pool (no `token_id` in the request)
/// or one token's pool. Every pool has the same subtree layout, so the handlers only differ in
/// the paths the selector hands them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShieldedPoolSelector {
    /// The main credit shielded pool.
    Credit,
    /// The shielded pool of the token with this id.
    Token([u8; 32]),
}

impl ShieldedPoolSelector {
    /// Resolves the optional `token_id` a request carries. A token pool is only addressable
    /// once token shielded pools exist (protocol version 14), and the id must be 32 bytes.
    pub(super) fn from_request(
        token_id: Option<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, QueryError> {
        match token_id {
            None => Ok(ShieldedPoolSelector::Credit),
            Some(token_id) => {
                if platform_version.protocol_version < TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
                {
                    return Err(QueryError::InvalidArgument(format!(
                        "token shielded pools are not active before protocol version {}",
                        TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
                    )));
                }
                let token_id: [u8; 32] = token_id.try_into().map_err(|token_id: Vec<u8>| {
                    QueryError::InvalidArgument(format!(
                        "token_id must be 32 bytes, got {}",
                        token_id.len()
                    ))
                })?;
                Ok(ShieldedPoolSelector::Token(token_id))
            }
        }
    }

    /// The pool subtree path.
    pub(super) fn pool_path_vec(&self) -> Vec<Vec<u8>> {
        match self {
            ShieldedPoolSelector::Credit => shielded_credit_pool_path_vec(),
            ShieldedPoolSelector::Token(token_id) => token_shielded_pool_path_vec(*token_id),
        }
    }

    /// The pool's nullifiers tree path.
    pub(super) fn nullifiers_path_vec(&self) -> Vec<Vec<u8>> {
        match self {
            ShieldedPoolSelector::Credit => shielded_credit_pool_nullifiers_path_vec(),
            ShieldedPoolSelector::Token(token_id) => {
                token_shielded_pool_nullifiers_path_vec(*token_id)
            }
        }
    }

    /// The pool's anchors tree path.
    pub(super) fn anchors_path_vec(&self) -> Vec<Vec<u8>> {
        match self {
            ShieldedPoolSelector::Credit => shielded_credit_pool_anchors_path_vec(),
            ShieldedPoolSelector::Token(token_id) => {
                token_shielded_pool_anchors_path_vec(*token_id)
            }
        }
    }

    /// The pool's canonical most-recent-anchor query.
    pub(super) fn latest_recorded_anchor_path_query(&self) -> PathQuery {
        match self {
            ShieldedPoolSelector::Credit => shielded_latest_recorded_anchor_path_query(),
            ShieldedPoolSelector::Token(token_id) => {
                token_shielded_pool_latest_recorded_anchor_path_query(*token_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_resolves_credit_pool_without_token_id() {
        let platform_version = PlatformVersion::latest();
        assert_eq!(
            ShieldedPoolSelector::from_request(None, platform_version).expect("credit"),
            ShieldedPoolSelector::Credit
        );
    }

    #[test]
    fn selector_resolves_token_pool_with_32_byte_token_id() {
        let platform_version = PlatformVersion::latest();
        let selector = ShieldedPoolSelector::from_request(Some(vec![7u8; 32]), platform_version)
            .expect("token pool");
        assert_eq!(selector, ShieldedPoolSelector::Token([7u8; 32]));
        assert_eq!(
            selector.nullifiers_path_vec(),
            token_shielded_pool_nullifiers_path_vec([7u8; 32])
        );
    }

    #[test]
    fn selector_rejects_wrong_length_token_id() {
        let platform_version = PlatformVersion::latest();
        assert!(matches!(
            ShieldedPoolSelector::from_request(Some(vec![7u8; 20]), platform_version),
            Err(QueryError::InvalidArgument(_))
        ));
    }

    #[test]
    fn selector_rejects_token_pools_before_activation() {
        let platform_version =
            PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
                .expect("previous version");
        assert!(matches!(
            ShieldedPoolSelector::from_request(Some(vec![7u8; 32]), platform_version),
            Err(QueryError::InvalidArgument(_))
        ));
    }
}
