//! Single bincode configuration + domain / version constants every
//! encoder in `secrets/wire/` uses.
//!
//! `WIRE_CONFIG` matches the platform-wide
//! `bincode::config::standard().with_big_endian().with_no_limit()`
//! (`rs-platform-serialization`) — big-endian for human-readable hex
//! dumps, varint integer encoding, no decode limit.
//!
//! Changing this constant invalidates every stored Tier-2 blob; the
//! golden-vector tests in [`super::envelope::tests`] catch any drift.

use bincode::config::{BigEndian, Configuration, NoLimit, Varint};

/// The one bincode config used to encode every wire byte under
/// `secrets/wire/` (envelope payload + the three AAD structs).
pub(crate) const WIRE_CONFIG: Configuration<BigEndian, Varint, NoLimit> =
    bincode::config::standard()
        .with_big_endian()
        .with_no_limit();

/// Tier-2 envelope wire version — bumped only on a breaking layout
/// change, independent of the vault `FORMAT_VERSION`. Bound into every
/// scheme-1 envelope's AAD so a forged version byte fails the tag.
pub(crate) const ENVELOPE_VERSION: u32 = 1;

/// Domain-separation tag leading the Tier-2 scheme-1 AAD. `-v2` marks the
/// wire-format break from the pre-bincode hand-rolled `PWSEV-TIER2-AAD-v1`.
pub(crate) const TIER2_DOMAIN_V2: &[u8] = b"PWSEV-TIER2-AAD-v2";

/// Domain-separation tag leading every vault `EntryAad`. Pre-bincode
/// `aad()` had no domain tag; bound here for symmetry + cross-context
/// disjointness with [`TIER2_DOMAIN_V2`] and [`VERIFY_DOMAIN_V2`].
pub(crate) const ENTRY_DOMAIN_V2: &[u8] = b"PWSV-ENTRY-AAD-v2";

/// Domain-separation tag leading every vault `VerifyAad`. Disjoint
/// from [`TIER2_DOMAIN_V2`] and [`ENTRY_DOMAIN_V2`] by **content past
/// the common prefix** (the three tags are NOT length-distinct —
/// TIER2 and VERIFY are both 18 bytes; ENTRY is 17). Pair-wise
/// byte-disjointness is pinned by the tests
/// `tier2_and_verify_aad_byte_disjoint`,
/// `tier2_and_entry_aad_byte_disjoint`, and
/// `entry_and_verify_aad_byte_disjoint`.
pub(crate) const VERIFY_DOMAIN_V2: &[u8] = b"PWSV-VERIFY-AAD-v2";
