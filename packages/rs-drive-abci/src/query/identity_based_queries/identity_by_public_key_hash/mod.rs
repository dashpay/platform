use crate::query_impl;

mod v0;

query_impl!(
    GetIdentityByPublicKeyHashesRequest,
    identity_based_queries,
    identity_by_public_key_hash,
    0
);
