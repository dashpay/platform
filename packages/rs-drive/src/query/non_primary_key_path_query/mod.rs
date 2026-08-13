//! Versioned lowering of a [`DriveDocumentQuery`](crate::query::DriveDocumentQuery)
//! over a secondary index into a grovedb path query. Dispatch lives on
//! `DriveDocumentQuery::get_non_primary_key_path_query` in the parent
//! module; the per-version implementations live here so already-live
//! behavior is isolated from later edits.

mod v0;
mod v1;
