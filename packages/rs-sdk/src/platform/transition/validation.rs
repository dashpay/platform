//! Re-export of the transport-free structure validation helper.
//!
//! The implementation moved to `dash-platform-queries`; broadcast paths in
//! this crate keep importing it from here. It returns the query core's
//! error type, which converts into [`crate::Error`] via `From` at the `?`
//! call sites.
pub(crate) use dash_platform_queries::transition::validation::ensure_valid_state_transition_structure;
