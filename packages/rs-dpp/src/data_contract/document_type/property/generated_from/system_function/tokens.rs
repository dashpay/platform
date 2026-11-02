//! `sys.tokens`: system functions of a token, named by its id, that read the
//! token's configuration (protocol version 15).
//!
//! Unlike the string transformations, these read state: the token's
//! configuration lives in the contract that defines it, and its conventions can
//! change after a document is written. So a property one of them generates is
//! generated when the document is created, and again only when a replace changes
//! the token it names; otherwise the document keeps the value it was created
//! with. The platform reads the value while it builds the write, and clients,
//! which hold no state, leave the property out for the platform to fill.

/// A `sys.tokens` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenFunction {
    /// `sys.tokens.decimals`: the token's decimals, from its conventions, as an
    /// integer from 0 to 255.
    Decimals,
}

impl TokenFunction {
    /// Every token function, in wire-name order.
    pub const ALL: [TokenFunction; 1] = [TokenFunction::Decimals];

    /// The wire name, the value of `generatedFrom.function`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            TokenFunction::Decimals => "sys.tokens.decimals",
        }
    }
}
