use super::index::StoredIndex;
use super::nested::MAX_STORED_DECODE_DEPTH;
use super::rules::StoredRule;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property_constraints::PropertyConstraint;
use crate::data_contract::document_type::{ContestedIndexFieldMatch, Index, LazyRegex};
use crate::tests::fixtures::get_dpns_data_contract_fixture;
use bincode::error::DecodeError;
use platform_version::version::PlatformVersion;

/// `present` under `depth` negations.
fn negated(depth: usize) -> PropertyConstraint {
    let mut rule = PropertyConstraint::Present("name".to_string());
    for _ in 0..depth {
        rule = PropertyConstraint::Not(Box::new(rule));
    }
    rule
}

/// `rule` stored, encoded, and decoded with the untrusted decoder, as from a proof.
fn round_trip(rule: &PropertyConstraint) -> Result<PropertyConstraint, DecodeError> {
    let config = bincode::config::standard().with_big_endian();
    let bytes = bincode::encode_to_vec(StoredRule::from(rule), config)
        .expect("expected the rule to encode");
    bincode::borrow_decode_from_slice_untrusted::<StoredRule, _>(&bytes, config)
        .map(|(stored, _)| stored.into())
}

/// A release build stores, decodes and drops a rule or a schema node nested to the bound within
/// a 2 MiB stack; an unoptimized build does not, so the test runs on a larger one.
#[test]
fn should_refuse_a_stored_rule_nested_past_the_decoding_bound() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let deepest = negated(MAX_STORED_DECODE_DEPTH);
            assert_eq!(
                round_trip(&deepest).expect("expected it to decode"),
                deepest
            );

            let error = round_trip(&negated(MAX_STORED_DECODE_DEPTH + 1))
                .expect_err("expected decoding to refuse it");
            assert!(error.to_string().contains("nesting depth"), "{error}");

            // The depth is back to zero: the next decode on this thread works
            assert_eq!(
                round_trip(&deepest).expect("expected it to decode"),
                deepest
            );
        })
        .expect("expected the thread to start")
        .join()
        .expect("expected the checks to pass");
}

/// A contested field's regex is compiled when its index is read back: a stored form that holds
/// one that does not compile is refused, where the lazy regex would have panicked on first
/// use. The schema parser refuses such a pattern at registration.
#[test]
fn should_refuse_a_stored_contested_regex_that_does_not_compile() {
    let contract =
        get_dpns_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
            .data_contract_owned();
    let (name, index) = contract
        .document_types()
        .values()
        .flat_map(|document_type| document_type.indexes().iter())
        .find(|(_, index)| index.contested_index.is_some())
        .map(|(name, index)| (name.clone(), index.clone()))
        .expect("expected DPNS to have a contested index");

    let read_back = Index::try_from(StoredIndex::from(&index)).expect("expected the index back");
    assert_eq!(read_back, index);

    let mut broken = index;
    let contested = broken
        .contested_index
        .as_mut()
        .expect("expected the contested index");
    let field = contested
        .field_matches
        .keys()
        .next()
        .expect("expected a contested field")
        .clone();
    contested.field_matches.insert(
        field,
        ContestedIndexFieldMatch::Regex(LazyRegex::new("(".to_string())),
    );
    let error = Index::try_from(StoredIndex::from(&broken))
        .expect_err("expected the stored regex to be refused");
    assert!(
        error.to_string().contains("does not compile"),
        "{name}: {error}"
    );
}
