//! PA-001b — Transfer with `output_change_address: None` vs `Some(addr)`.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-001b.
//! Priority: P2.
//!
//! ## Status
//!
//! `BLOCKED — feature missing in production.` See spec status field
//! and Found-019 (sibling Found-bug pin documenting the spec drift).
//!
//! The spec describes driving `PlatformAddressWallet::transfer` with
//! an `output_change_address: Option<PlatformAddress>` parameter that
//! does not exist in the production signature
//! (`packages/rs-platform-wallet/src/wallet/platform_addresses/transfer.rs:31`):
//!
//! ```rust,ignore
//! pub async fn transfer<S: Signer<PlatformAddress> + Send + Sync>(
//!     &self,
//!     account_index: u32,
//!     input_selection: InputSelection,
//!     outputs: BTreeMap<PlatformAddress, Credits>,
//!     fee_strategy: AddressFundsFeeStrategy,
//!     platform_version: Option<&PlatformVersion>,
//!     address_signer: &S,
//! ) -> Result<PlatformAddressChangeSet, PlatformWalletError>
//! ```
//!
//! Under the current shape, "change" semantics are implicit:
//!
//! - `InputSelection::Auto`: the auto-selector consumes input balance
//!   to cover `Σ outputs` exactly under the post-fix `Σ inputs ==
//!   Σ outputs` invariant. There is no separate "change output", so
//!   no `output_change_address` to route — residual stays on the
//!   selected input addresses.
//! - `InputSelection::Explicit(map)`: the caller declares the
//!   consumed amount per input directly. Any residual stays on the
//!   input.
//!
//! PA-001b is therefore not a missing TEST — it's a missing FEATURE.
//! Surfaced as a Found-bug pin in the spec; this stub stays
//! `#[ignore]`'d until either the production API gains an explicit
//! change-address parameter or the spec entry is removed.

#[tokio_shared_rt::test(shared)]
#[ignore = "BLOCKED — feature missing in production: \
            PlatformAddressWallet::transfer has no output_change_address \
            parameter. See TEST_SPEC.md PA-001b status field and the \
            Found-NNN entry for the spec/impl drift."]
async fn pa_001b_change_address_branch() {
    panic!(
        "PA-001b is BLOCKED on a missing production API. \
         The spec describes an `output_change_address: Option<PlatformAddress>` \
         parameter on `PlatformAddressWallet::transfer` that does not exist in \
         `packages/rs-platform-wallet/src/wallet/platform_addresses/transfer.rs:31`. \
         See TEST_SPEC.md → PA-001b → **Status** and the corresponding \
         Found-NNN entry. This `#[ignore]` is intentional; remove it only \
         once the production API gains the parameter."
    );
}
