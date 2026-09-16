//! Error constructors shared by the history reads.

use crate::error::{drive::DriveError, query::QuerySyntaxError, Error};

pub(crate) fn invalid(message: &str) -> Error {
    Error::Query(QuerySyntaxError::Unsupported(message.to_owned()))
}

pub(super) fn corrupt(message: &'static str) -> Error {
    Error::Drive(DriveError::CorruptedDocumentPath(message))
}
