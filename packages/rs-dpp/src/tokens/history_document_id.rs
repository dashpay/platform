//! The id of the document a token transition writes into the token history contract.

use crate::balances::credits::TokenAmount;
use crate::document::Document;
use crate::prelude::IdentityNonce;
use platform_value::Identifier;

/// The id of the history document of a token event of `document_type_name` (one of the token
/// history contract's types, `TokenEvent::associated_document_type_name`) that the transition
/// of `owner_id` with `owner_nonce` on `token_id` writes. The writer, the settlement of a
/// random amount and the proof verifier all find the document by it.
///
/// A random transfer or mint (protocol version 15) passes `random_range`, the least and the
/// most it moves: its id hashes both after the nonce, so the history document of a transfer
/// or mint of a fixed amount, or of one drawn from another range, has another id and can not
/// stand for it in a proof. Every other event passes `None` and keeps the id it always had.
pub fn token_history_document_id(
    token_id: &Identifier,
    owner_id: &Identifier,
    document_type_name: &str,
    owner_nonce: IdentityNonce,
    random_range: Option<(TokenAmount, TokenAmount)>,
) -> Identifier {
    let mut entropy = owner_nonce.to_be_bytes().to_vec();
    if let Some((minimum, maximum)) = random_range {
        entropy.extend_from_slice(&minimum.to_be_bytes());
        entropy.extend_from_slice(&maximum.to_be_bytes());
    }
    Document::generate_document_id_v0(
        token_id,
        owner_id,
        format!("history_{}", document_type_name).as_str(),
        entropy.as_slice(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: [u8; 32] = [1; 32];
    const OWNER: [u8; 32] = [2; 32];

    fn id(random_range: Option<(TokenAmount, TokenAmount)>) -> Identifier {
        token_history_document_id(
            &Identifier::from(TOKEN),
            &Identifier::from(OWNER),
            "transfer",
            7,
            random_range,
        )
    }

    #[test]
    fn should_keep_the_id_of_an_event_without_a_random_amount() {
        assert_eq!(
            id(None),
            Document::generate_document_id_v0(
                &Identifier::from(TOKEN),
                &Identifier::from(OWNER),
                "history_transfer",
                7u64.to_be_bytes().as_slice(),
            )
        );
    }

    #[test]
    fn should_give_a_random_amount_an_id_of_its_own_range() {
        let random = id(Some((10, 500)));
        assert_ne!(random, id(None));
        assert_ne!(random, id(Some((10, 501))));
        assert_ne!(random, id(Some((11, 500))));
        assert_eq!(random, id(Some((10, 500))));
    }
}
