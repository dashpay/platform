use crate::query_impl;

mod v0;

query_impl!(GetIdentityRequest, identity_based_queries, identity, 0);
