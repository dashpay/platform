//! A page of the entries of a private document type's store (`getPrivateDocumentEntries`,
//! protocol version 15): the type's fixed-size encrypted entries in append order, each with
//! the height and time of the block that appended it, and how many entries the store holds.
//!
//! Both are Drive's own types, which its proof verifier returns.

pub use drive::drive::document::private_document_store::{
    PrivateDocumentEntries, PrivateDocumentEntry,
};
