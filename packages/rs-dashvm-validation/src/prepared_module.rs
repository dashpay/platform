//! Preparation of one module: admission, instrumentation, re-validation, hashing.

use crate::admission::{validate_prepared, validate_submitted};
use crate::bundle::{ModuleName, PreparedModule};
use crate::errors::ModuleError;
use crate::hashing::{CanonicalHash, PreparedHash};
use crate::instrumentation::rewrite::instrument;
use crate::profile::PreparationProfile;

/// Turns submitted canonical bytes into a prepared module.
///
/// Steps, cheap first: the byte cap, full validation under the admitted feature set with
/// structural measurement, the import and export rules, the frame-cost plan, the rewrite, the
/// re-validation and provenance check of the output, and finally the two hashes. The canonical
/// hash is taken over the submitted bytes exactly as received, before any transformation; the
/// prepared hash only over bytes that passed [`validate_prepared`].
pub fn prepare_module(
    name: ModuleName,
    canonical_bytes: &[u8],
    profile: &PreparationProfile,
) -> Result<PreparedModule, ModuleError> {
    let submitted = validate_submitted(canonical_bytes, profile)?;
    let (prepared_bytes, instrumentation) =
        instrument(canonical_bytes, &submitted.facts, &submitted.plan)?;
    let prepared_facts = validate_prepared(&prepared_bytes, profile, &submitted, &instrumentation)?;
    let mut structure = submitted.facts.structure.clone();
    structure.prepared_bytes = prepared_facts.structure.canonical_bytes;
    Ok(PreparedModule {
        name,
        canonical_hash: CanonicalHash::of(canonical_bytes),
        prepared_hash: PreparedHash::of(&prepared_bytes),
        prepared_bytes,
        preparation_generation: profile.generation,
        interface: submitted.interface,
        structure,
        initialization: submitted.facts.initialization,
        instrumentation,
    })
}
