//! The capability catalogue: what a contract may require, and whether the
//! native host supports it today.

use core::fmt;

use super::collections::TypedCollectionKind;

/// Support status of a capability in the native host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityStatus {
    /// Supported by the native host today; a manifest requesting it can be
    /// deployed.
    Native,
    /// Specified, declarable, but without a native implementation yet. The
    /// validator accepts the declaration; the build crate reports it as a
    /// native gap and refuses to call the manifest deployable.
    PendingNative,
    /// Catalogued but disabled: the declaration is rejected until the
    /// capability's semantics are specified. Today only the private document
    /// store, whose encryption, key control, query visibility and proof
    /// behaviour are not yet specified.
    InterfaceDisabled,
}

/// A capability a contract requires, either explicitly (`requires = [...]`)
/// or derived from its declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityRequirement {
    /// The native access-control primitive. Explicit.
    Acl,
    /// The native randomness primitive. Explicit.
    Randomness,
    /// `write = "contract"`: only the contract's own code may create
    /// documents. Derived.
    ContractWrites,
    /// A rule with a native guard expression. Derived.
    NativeGuards,
    /// A rule with a read-only WASM predicate. Derived.
    WasmPredicates,
    /// A typed specialized collection of the given kind. Derived.
    TypedCollections(TypedCollectionKind),
    /// `store = "private"`. Derived.
    PrivateStore,
    /// Stored receipts (the default policy). Derived.
    StoredReceipts,
    /// At least one entry, which needs the DashVM runtime. Derived.
    Entries,
    /// More than one module, which needs bundle linking. Derived.
    Modules,
}

impl CapabilityRequirement {
    /// Whether the author writes this requirement (as opposed to the validator
    /// deriving it).
    pub fn is_explicit(&self) -> bool {
        matches!(
            self,
            CapabilityRequirement::Acl | CapabilityRequirement::Randomness
        )
    }

    /// The catalogue status of the requirement.
    pub fn status(&self) -> CapabilityStatus {
        match self {
            CapabilityRequirement::PrivateStore => CapabilityStatus::InterfaceDisabled,
            CapabilityRequirement::Acl
            | CapabilityRequirement::Randomness
            | CapabilityRequirement::ContractWrites
            | CapabilityRequirement::NativeGuards
            | CapabilityRequirement::WasmPredicates
            | CapabilityRequirement::TypedCollections(_)
            | CapabilityRequirement::StoredReceipts
            | CapabilityRequirement::Entries
            | CapabilityRequirement::Modules => CapabilityStatus::PendingNative,
        }
    }
}

impl fmt::Display for CapabilityRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityRequirement::Acl => f.write_str("acl"),
            CapabilityRequirement::Randomness => f.write_str("randomness"),
            CapabilityRequirement::ContractWrites => f.write_str("contract writes"),
            CapabilityRequirement::NativeGuards => f.write_str("native guards"),
            CapabilityRequirement::WasmPredicates => f.write_str("wasm predicates"),
            CapabilityRequirement::TypedCollections(kind) => {
                write!(f, "typed collections ({kind})")
            }
            CapabilityRequirement::PrivateStore => f.write_str("private store"),
            CapabilityRequirement::StoredReceipts => f.write_str("stored receipts"),
            CapabilityRequirement::Entries => f.write_str("entries"),
            CapabilityRequirement::Modules => f.write_str("modules"),
        }
    }
}

/// Whether the host stores a receipt for every outer invocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReceiptPolicy {
    /// One bounded receipt per outer invocation, the default.
    #[default]
    Stored,
    /// No receipts.
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_the_private_store_catalogued_but_disabled() {
        assert_eq!(
            CapabilityRequirement::PrivateStore.status(),
            CapabilityStatus::InterfaceDisabled
        );
        assert!(!CapabilityRequirement::PrivateStore.is_explicit());
    }

    #[test]
    fn should_mark_acl_and_randomness_explicit_and_pending() {
        for capability in [
            CapabilityRequirement::Acl,
            CapabilityRequirement::Randomness,
        ] {
            assert!(capability.is_explicit());
            assert_eq!(capability.status(), CapabilityStatus::PendingNative);
        }
    }

    #[test]
    fn should_default_receipts_to_stored() {
        assert_eq!(ReceiptPolicy::default(), ReceiptPolicy::Stored);
    }
}
