use super::nested::MAX_STORED_DECODE_DEPTH;
use super::rules::StoredRule;
use crate::data_contract::document_type::property_constraints::PropertyConstraint;
use bincode::error::DecodeError;

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
