//! Contract-author SDK for DashVM: the declaration model behind
//! `#[persistent]`, `#[index]`, `#[rule]` and `#[entry]`, the attribute
//! grammar those macros implement, the diagnostics they report, and the
//! canonical manifest a contract package publishes.
//!
//! This crate specifies the author-facing model. It carries no proc macros, no
//! host context and no runtime; those are later deliverables of the same
//! workstream and consume the types defined here:
//!
//! - [`grammar`] is the attribute grammar as data: which attributes exist, which
//!   options they take and which values are legal. The proc macros, this
//!   crate's validator and the book chapter share this single table.
//! - [`declare`] is the typed declaration model: [`declare::ContractDeclaration`]
//!   gathers collections, indexes, rules, entries, modules, interfaces, typed
//!   collections and capability requirements from attributes and from builders.
//! - [`validate`] turns a declaration into a [`manifest::CanonicalManifest`] or
//!   into a complete list of [`validate::Diagnostic`]s. It owns grammar,
//!   identity, boundedness, conflict and SDK-semantic checks. Native numeric
//!   limits (index counts, name lengths, indexed string sizes) are deliberately
//!   not duplicated: Dash Platform Protocol enforces them once and the build
//!   crate surfaces them.
//! - [`manifest`] is the sorted, order-independent canonical manifest. It has no
//!   wire encoding yet; the encoding and the numeric identifiers are allocated
//!   by the ABI work.
//! - [`persistence`] states the persistence semantics as enums so that generated
//!   wrappers and the documentation cannot drift from the confirmed policy:
//!   detached values never save, explicit operations and successful mutable
//!   receivers stage writes, nothing saves on `Drop`.
//!
//! The crate compiles without `std` (`--no-default-features`) on
//! `wasm32v1-none`, which is how guest packages depend on it.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

pub mod declare;
pub mod grammar;
pub mod identity;
pub mod manifest;
pub mod persistence;
pub mod prelude;
pub mod validate;
