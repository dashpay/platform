//! Versioned lowering of a [`DriveDocumentQuery`](crate::query::DriveDocumentQuery)
//! over a secondary index into a grovedb path query. Dispatch lives on
//! `DriveDocumentQuery::get_non_primary_key_path_query` in the parent
//! module; the per-version implementations live here so already-live
//! behavior is isolated from later edits. From v1 on, the lowering only
//! routes by query shape — the shape lowerings themselves are versioned
//! methods in the `single_in_path_query` and `multiple_in_path_query`
//! submodules.

mod multiple_in_path_query;
mod single_in_path_query;
mod v0;
mod v1;
