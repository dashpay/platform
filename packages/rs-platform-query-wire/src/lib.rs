//! Shared wire→drive decoding for Dash Platform queries.
//!
//! This crate is the single home of the decoders that map query
//! wire-proto types (from `dapi-grpc`) onto `drive::query` types. It
//! is consumed by the server (rs-drive-abci decodes incoming requests
//! through it) and is intended for client-side proof verifiers, so
//! the server's and a verifier's interpretation of the same request
//! bytes cannot drift.
//!
//! Scope is deliberately narrow: decode only. No transport, no proof
//! verification, no networking — the dependency graph is a strict
//! subset of what rs-drive-abci already carries.

pub mod proto_conversions;
