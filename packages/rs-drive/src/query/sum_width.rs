//! Reading the sums of document indexes at either width they are kept in.
//!
//! A document type keeps its sums in trees with a 64-bit total or, from protocol version 15
//! and for a summed property declared past ±2^27, in trees with a 128-bit one
//! (`DocumentTypeV2Getters::uses_big_sums`). The sum results the unproven reads return carry
//! 64 bits, so a sum they read from a 128-bit tree is returned when it fits them and refused
//! when it does not, rather than read as the zero GroveDB's 64-bit accessors give such a
//! tree. The sum and average proof verifiers read a proved element's sum at 128 bits
//! (`Element::big_sum_value_or_default`) and return it whole; a ranked entry carries 64 bits,
//! so the ranked verifiers refuse a wider one with [`proved_sum_in_64_bits`].
//!
//! For every element that holds no 128-bit sum, `Element::big_sum_value_or_default` is
//! `Element::sum_value_or_default` widened and `Element::count_big_sum_value_or_default` is
//! `Element::count_sum_value_or_default` with its sum widened, so the functions here return
//! what the 64-bit accessors return wherever no such element exists, which is every state
//! before protocol version 15.

use crate::error::proof::ProofError;
#[cfg(feature = "server")]
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
#[cfg(feature = "server")]
use grovedb::Element;

/// `sum` as a sum result carries it. A sum past 64 bits is refused: no result can carry it
/// yet.
#[cfg(feature = "server")]
pub(crate) fn sum_in_64_bits(sum: i128) -> Result<i64, Error> {
    i64::try_from(sum).map_err(|_| {
        Error::Query(QuerySyntaxError::Unsupported(
            "the sum does not fit the 64 bits a sum result carries".to_string(),
        ))
    })
}

/// The sum `element` holds or contributes, at either width.
#[cfg(feature = "server")]
pub(crate) fn element_sum(element: &Element) -> Result<i64, Error> {
    sum_in_64_bits(element.big_sum_value_or_default())
}

/// The count and the sum `element` holds or contributes, at either width.
#[cfg(feature = "server")]
pub(crate) fn element_count_and_sum(element: &Element) -> Result<(u64, i64), Error> {
    let (count, sum) = element.count_big_sum_value_or_default();
    Ok((count, sum_in_64_bits(sum)?))
}

/// `sum_in_64_bits` for a sum read from a proof that reconstructs `root_hash`. A sum past
/// 64 bits is refused with that root ([`ProofError::SumPastResultWidth`]), so the caller can
/// authenticate it before reporting the refusal: every honest node proves the same sum, and
/// a node is not to be taken for a wrong one for it.
pub(crate) fn proved_sum_in_64_bits(sum: i128, root_hash: [u8; 32]) -> Result<i64, Error> {
    i64::try_from(sum).map_err(|_| Error::Proof(ProofError::SumPastResultWidth { root_hash }))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn should_read_a_64_bit_element_as_its_own_accessors_do() {
        for element in [
            Element::new_sum_item(-7),
            Element::new_item(vec![1]),
            Element::empty_tree(),
            Element::SumTree(None, 9, None),
            Element::CountTree(None, 4, None),
            Element::CountSumTree(None, 4, -9, None),
            Element::ProvableSumTree(None, i64::MAX, None),
            Element::ProvableCountTree(None, 4, None),
            Element::ProvableCountSumTree(None, 4, i64::MIN, None),
            Element::ProvableCountProvableSumTree(None, 4, 9, None),
            Element::NonCounted(Box::new(Element::SumTree(None, 9, None))),
            Element::NotSummed(Box::new(Element::CountSumTree(None, 4, 9, None))),
            Element::NotCountedOrSummed(Box::new(Element::CountSumTree(None, 4, 9, None))),
        ] {
            assert_eq!(
                element_sum(&element).expect("expected the sum"),
                element.sum_value_or_default(),
                "{element:?}"
            );
            assert_eq!(
                element_count_and_sum(&element).expect("expected the count and the sum"),
                element.count_sum_value_or_default(),
                "{element:?}"
            );
        }
    }

    #[test]
    fn should_read_a_128_bit_sum_that_fits_64_bits() {
        assert_eq!(
            element_sum(&Element::ProvableBigSumTree(None, 12, None)).expect("expected the sum"),
            12
        );
        assert_eq!(
            element_count_and_sum(&Element::CountBigSumTree(None, 3, -12, None))
                .expect("expected the count and the sum"),
            (3, -12)
        );
        assert_eq!(
            element_count_and_sum(&Element::ProvableCountProvableBigSumTree(
                None,
                3,
                i128::from(i64::MAX),
                None
            ))
            .expect("expected the count and the sum"),
            (3, i64::MAX)
        );
    }

    #[test]
    fn should_refuse_a_sum_past_64_bits() {
        for element in [
            Element::BigSumTree(None, i128::from(i64::MAX) + 1, None),
            Element::CountBigSumTree(None, 2, i128::from(i64::MIN) - 1, None),
        ] {
            assert!(matches!(
                element_sum(&element),
                Err(Error::Query(QuerySyntaxError::Unsupported(_)))
            ));
            assert!(matches!(
                element_count_and_sum(&element),
                Err(Error::Query(QuerySyntaxError::Unsupported(_)))
            ));
        }
    }
}
