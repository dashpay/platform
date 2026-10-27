use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::document::{Document, DocumentV0, DocumentV0Getters};
use crate::fee::Credits;
use crate::prelude::Revision;
use crate::tokens::token_order_side::TokenOrderSide;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

/// The token market contract's property names of an order.
pub mod property_names {
    pub const TOKEN_ID: &str = "tokenId";
    pub const PRICE: &str = "price";
    pub const AMOUNT: &str = "amount";
    pub const REMAINING: &str = "remaining";
    pub const ESCROW: &str = "escrow";
    pub const DECIMALS: &str = "decimals";
}

/// An open order of the token marketplace (protocol version 15), as the token market contract
/// stores it: a listing (`Ask`) of tokens for sale, or an offer (`Bid`) to buy them.
///
/// The price is in credits per whole token: 10 to the power of `decimals` base units, the token's
/// decimals when the order was placed. Every fill prices the order at that scale, so a later
/// change to the token's decimals changes neither what a listing costs nor what an offer pays.
/// A fill pays the change in the order's rounded running total, so splitting an order into
/// several fills never changes what it costs in all: a listing's buyers pay
/// `ceil((filled + q) * price / unit) - ceil(filled * price / unit)` for `q` base units, and an
/// offer's sellers receive the same difference rounded down. The rounding favours the order's
/// maker, and is never more than one credit per fill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenOrder {
    /// The order id: its document id in the token market contract and the key of its escrow.
    pub id: Identifier,
    /// A listing or an offer.
    pub side: TokenOrderSide,
    /// The identity that placed the order.
    pub maker_id: Identifier,
    /// The token the order is for.
    pub token_id: Identifier,
    /// The price, in credits per whole token.
    pub price: Credits,
    /// The amount, in base units, listed or wanted when the order was placed.
    pub amount: TokenAmount,
    /// The amount still open.
    pub remaining: TokenAmount,
    /// For an offer, the credits still held in escrow; 0 for a listing, whose escrow holds
    /// tokens (`remaining`).
    pub escrow: Credits,
    /// The token's decimals when the order was placed, the scale of its price: what the token
    /// market contract's `generatedFrom` (`sys.tokens.decimals` of `tokenId`) generates, and
    /// keeps for the life of the order.
    pub decimals: u8,
}

/// 10 to the power of `decimals`, the base units in one whole token, or `None` above
/// `u128::MAX` (39 decimals or more). A token's decimals are a `u8`, so the unit may not fit.
fn unit(decimals: u8) -> Option<u128> {
    10u128.checked_pow(decimals as u32)
}

/// What `filled` base units at `price` credits per whole token cost, rounded up or down, with
/// `unit` base units in a whole token (`None` when that is more than `u128` holds).
fn total_at(filled: TokenAmount, price: Credits, unit: Option<u128>, round_up: bool) -> u128 {
    // At most `i64::MAX * i64::MAX`, under `u128::MAX`.
    let value = filled as u128 * price as u128;
    match unit {
        Some(unit) if round_up => value.div_ceil(unit),
        Some(unit) => value / unit,
        // A whole token is more base units than `u128` holds, so more than `value`: a nonzero
        // total is under one credit, 1 rounded up and 0 rounded down.
        None if round_up => u128::from(value > 0),
        None => 0,
    }
}

impl TokenOrder {
    /// The amount filled so far.
    pub fn filled(&self) -> TokenAmount {
        self.amount.saturating_sub(self.remaining)
    }

    /// The amount a fill asking for between `min_amount` and `max_amount` takes, or `None` when
    /// no amount in that range keeps the token's rules. `min_fill` is the token's minimum fill
    /// (`TokenTradeMode::Tradeable`) when the fill executes.
    ///
    /// The fill takes as much as it may: everything open, up to `max_amount`. When that would
    /// leave less than `min_fill` open, it takes less, leaving exactly `min_fill`, if that still
    /// takes at least `min_fill` and `min_amount`. A fill that does not take everything open takes
    /// at least `min_fill`; one that takes everything open may take less, so an order left under
    /// a minimum the token raised since is still filled whole.
    pub fn fill_amount(
        &self,
        min_amount: TokenAmount,
        max_amount: TokenAmount,
        min_fill: TokenAmount,
    ) -> Option<TokenAmount> {
        let mut take = max_amount.min(self.remaining);
        if take < self.remaining {
            let left = self.remaining - take;
            if left < min_fill {
                take = self.remaining.checked_sub(min_fill)?;
            }
            if take < min_fill {
                return None;
            }
        }
        if take == 0 || take < min_amount {
            return None;
        }
        Some(take)
    }

    /// The credits a fill of `fill_amount` moves: what a listing's buyer pays, rounded up, or
    /// what an offer's seller receives, rounded down, as the change in the order's running total
    /// at the order's own `decimals`.
    pub fn fill_credits(&self, fill_amount: TokenAmount) -> Result<Credits, ProtocolError> {
        let unit = unit(self.decimals);
        let filled = self.filled();
        let filled_after = filled
            .checked_add(fill_amount)
            .ok_or(ProtocolError::Overflow(
                "a fill takes more than an order holds",
            ))?;
        let round_up = matches!(self.side, TokenOrderSide::Ask);
        let before = total_at(filled, self.price, unit, round_up);
        let after = total_at(filled_after, self.price, unit, round_up);
        Credits::try_from(after - before)
            .map_err(|_| ProtocolError::Overflow("a fill moves more credits than fit in u64"))
    }

    /// What an order of `amount` base units at `price` credits per whole token is worth, rounded
    /// up, for a token with `decimals` decimals: everything its buyers pay for a listing, and the
    /// escrow an offer holds. `None` when it is more credits than an order holds (`i64::MAX`).
    pub fn order_value(amount: TokenAmount, price: Credits, decimals: u8) -> Option<Credits> {
        let value = total_at(amount, price, unit(decimals), true);
        if value > i64::MAX as u128 {
            None
        } else {
            Some(value as Credits)
        }
    }

    /// What an offer holds in escrow after its fills: what it is worth, rounded up, less what its
    /// sellers were paid, the running total of what is filled rounded down. `None` for a listing,
    /// whose escrow holds tokens, and for an offer worth more than an order holds.
    pub fn escrow_after_fills(&self) -> Option<Credits> {
        if self.side != TokenOrderSide::Bid {
            return None;
        }
        let value = Self::order_value(self.amount, self.price, self.decimals)?;
        let paid = total_at(self.filled(), self.price, unit(self.decimals), false);
        (value as u128)
            .checked_sub(paid)
            .and_then(|left| Credits::try_from(left).ok())
    }

    /// Reads an order of `side` from its token market contract document.
    pub fn from_document(document: &Document, side: TokenOrderSide) -> Result<Self, ProtocolError> {
        let properties = document.properties();
        let integer = |name: &str| -> Result<u64, ProtocolError> {
            properties
                .get(name)
                .ok_or_else(|| {
                    ProtocolError::CorruptedCodeExecution(format!(
                        "a token order document has no {name}"
                    ))
                })?
                .to_integer::<u64>()
                .map_err(ProtocolError::ValueError)
        };
        let token_id = properties
            .get(property_names::TOKEN_ID)
            .ok_or_else(|| {
                ProtocolError::CorruptedCodeExecution(
                    "a token order document has no tokenId".to_string(),
                )
            })?
            .to_identifier()
            .map_err(ProtocolError::ValueError)?;
        let escrow = match side {
            TokenOrderSide::Ask => 0,
            TokenOrderSide::Bid => integer(property_names::ESCROW)?,
        };
        let decimals = u8::try_from(integer(property_names::DECIMALS)?).map_err(|_| {
            ProtocolError::CorruptedCodeExecution(
                "a token order document's decimals are above 255".to_string(),
            )
        })?;
        Ok(TokenOrder {
            id: document.id(),
            side,
            maker_id: document.owner_id(),
            token_id,
            price: integer(property_names::PRICE)?,
            amount: integer(property_names::AMOUNT)?,
            remaining: integer(property_names::REMAINING)?,
            escrow,
            decimals,
        })
    }

    /// The order's properties as the token market contract stores them: every integer a signed
    /// 8-byte value (the contract stores integers as i64).
    fn properties(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let signed = |value: u64| -> Result<Value, ProtocolError> {
            i64::try_from(value)
                .map(Value::I64)
                .map_err(|_| ProtocolError::Overflow("a token order integer is above i64::MAX"))
        };
        let mut properties = BTreeMap::from([
            (
                property_names::TOKEN_ID.to_string(),
                Value::Identifier(self.token_id.to_buffer()),
            ),
            (property_names::PRICE.to_string(), signed(self.price)?),
            (property_names::AMOUNT.to_string(), signed(self.amount)?),
            (
                property_names::REMAINING.to_string(),
                signed(self.remaining)?,
            ),
            (
                property_names::DECIMALS.to_string(),
                Value::I64(self.decimals as i64),
            ),
        ]);
        if self.side == TokenOrderSide::Bid {
            properties.insert(property_names::ESCROW.to_string(), signed(self.escrow)?);
        }
        Ok(properties)
    }

    /// The document of a new order, placed at `block_info`.
    pub fn new_document(&self, block_info: &BlockInfo) -> Result<Document, ProtocolError> {
        Ok(DocumentV0 {
            contract_version: None,
            id: self.id,
            owner_id: self.maker_id,
            properties: self.properties()?,
            revision: Some(1),
            created_at: Some(block_info.time_ms),
            updated_at: Some(block_info.time_ms),
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
        }
        .into())
    }

    /// The document `stored` after a partial fill left this order's `remaining` (and, for an
    /// offer, its `escrow`): the same document at the next revision, updated at `block_info`.
    pub fn updated_document(
        &self,
        stored: &Document,
        block_info: &BlockInfo,
    ) -> Result<Document, ProtocolError> {
        let revision: Revision = stored.revision().unwrap_or(1).saturating_add(1);
        let mut document = stored.clone();
        let Document::V0(ref mut v0) = document;
        v0.properties = self.properties()?;
        v0.revision = Some(revision);
        v0.updated_at = Some(block_info.time_ms);
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(side: TokenOrderSide, amount: TokenAmount, remaining: TokenAmount) -> TokenOrder {
        TokenOrder {
            id: Identifier::from([1; 32]),
            side,
            maker_id: Identifier::from([2; 32]),
            token_id: Identifier::from([3; 32]),
            price: 333,
            amount,
            remaining,
            escrow: 0,
            decimals: 2,
        }
    }

    #[test]
    fn should_take_everything_open_up_to_the_fill_maximum() {
        let listing = order(TokenOrderSide::Ask, 1000, 1000);
        assert_eq!(listing.fill_amount(1, 400, 1), Some(400));
        assert_eq!(listing.fill_amount(1, 5000, 1), Some(1000));
        assert_eq!(listing.fill_amount(1001, 5000, 1), None);
    }

    #[test]
    fn should_leave_at_least_the_minimum_fill_open() {
        let listing = order(TokenOrderSide::Ask, 1000, 1000);
        // 950 would leave 50: the fill takes 900 and leaves 100.
        assert_eq!(listing.fill_amount(1, 950, 100), Some(900));
        // Unless the fill asked for more than that.
        assert_eq!(listing.fill_amount(901, 950, 100), None);
        // A fill under the minimum fill that does not take everything is refused.
        assert_eq!(listing.fill_amount(1, 50, 100), None);
        // Everything open is always takeable, even under a minimum the token raised since.
        let tail = order(TokenOrderSide::Ask, 1000, 60);
        assert_eq!(tail.fill_amount(1, 1000, 100), Some(60));
    }

    #[test]
    fn should_sell_a_listing_under_the_minimum_fill_only_whole() {
        let listing = order(TokenOrderSide::Ask, 1000, 1000);
        assert_eq!(listing.fill_amount(1, 999, 1000), None);
        assert_eq!(listing.fill_amount(1, 1000, 1000), Some(1000));
    }

    /// 2 decimals at 333 credits per token is 3.33 credits per base unit.
    #[test]
    fn should_charge_the_running_total_so_splitting_never_changes_the_cost() {
        let mut listing = order(TokenOrderSide::Ask, 3, 3);
        let mut paid = vec![];
        for _ in 0..3 {
            let credits = listing.fill_credits(1).expect("expected the cost");
            paid.push(credits);
            listing.remaining -= 1;
        }
        assert_eq!(paid, vec![4, 3, 3]);
        let whole = order(TokenOrderSide::Ask, 3, 3);
        assert_eq!(whole.fill_credits(3).expect("expected the cost"), 10);
    }

    #[test]
    fn should_round_an_offers_payouts_down_and_keep_them_within_its_escrow() {
        let escrow = TokenOrder::order_value(1001, 333, 2).expect("expected a value");
        assert_eq!(escrow, 3334);
        let mut offer = order(TokenOrderSide::Bid, 1001, 1001);
        let mut paid = 0;
        while offer.remaining > 0 {
            let take = offer.remaining.min(7);
            paid += offer.fill_credits(take).expect("expected the payout");
            offer.remaining -= take;
        }
        assert_eq!(paid, 3333);
        assert!(paid <= escrow);
    }

    /// An order prices its fills at the decimals it records, whatever the token's are now: the
    /// same price and amount cost 100 times more at 0 decimals than at 2.
    #[test]
    fn should_price_fills_at_the_orders_own_decimals() {
        let mut listing = order(TokenOrderSide::Ask, 100, 100);
        listing.price = 100;
        assert_eq!(listing.fill_credits(10).expect("expected the cost"), 10);
        listing.decimals = 0;
        assert_eq!(listing.fill_credits(10).expect("expected the cost"), 1_000);
    }

    /// The escrow an offer holds after any sequence of sales is its value less the running total
    /// paid, so it follows from what is filled alone.
    #[test]
    fn should_know_what_an_offer_holds_after_its_fills() {
        let mut offer = order(TokenOrderSide::Bid, 1001, 1001, 1);
        offer.escrow = TokenOrder::order_value(1001, 333, 2).expect("expected a value");
        assert_eq!(offer.escrow_after_fills(), Some(offer.escrow));
        while offer.remaining > 0 {
            let take = offer.remaining.min(7);
            offer.escrow -= offer.fill_credits(take).expect("expected the payout");
            offer.remaining -= take;
            assert_eq!(offer.escrow_after_fills(), Some(offer.escrow));
        }
        assert_eq!(offer.escrow, 1, "the dust the buyer gets back");
        assert_eq!(
            order(TokenOrderSide::Ask, 10, 10, 1).escrow_after_fills(),
            None
        );
    }

    #[test]
    fn should_refuse_an_order_worth_more_than_an_order_holds() {
        assert_eq!(TokenOrder::order_value(i64::MAX as u64, 2, 0), None);
        assert_eq!(
            TokenOrder::order_value(i64::MAX as u64, 1, 0),
            Some(i64::MAX as u64)
        );
    }

    /// A token's decimals are a \`u8\`. From 39 decimals a whole token is more base units than
    /// \`u128\` holds, and more than any amount times any price, so an order is worth one credit
    /// rounded up (a listing's first fill pays it) and nothing rounded down (an offer's sales
    /// are paid nothing).
    #[test]
    fn should_price_orders_of_tokens_with_39_or_more_decimals() {
        let max = i64::MAX as u64;
        for decimals in [39, 255] {
            assert_eq!(TokenOrder::order_value(1, 1, decimals), Some(1));
            assert_eq!(TokenOrder::order_value(max, max, decimals), Some(1));

            let mut listing = order(TokenOrderSide::Ask, 1_000, 1_000);
            listing.decimals = decimals;
            listing.price = max;
            assert_eq!(listing.fill_credits(400).ok(), Some(1));
            listing.remaining = 600;
            assert_eq!(listing.fill_credits(600).ok(), Some(0));

            let mut offer = order(TokenOrderSide::Bid, 1_000, 1_000);
            offer.decimals = decimals;
            offer.price = max;
            assert_eq!(offer.fill_credits(1_000).ok(), Some(0));
            offer.escrow = 1;
            assert_eq!(
                offer.escrow_after_fills(),
                Some(1),
                "the credit goes back when it closes"
            );
        }
        // 38 decimals still fit: 10^38 base units in a whole token.
        assert_eq!(
            TokenOrder::order_value(max, max, 38),
            Some((max as u128 * max as u128).div_ceil(10u128.pow(38)) as u64)
        );
    }

    #[test]
    fn should_round_trip_through_its_document() {
        let mut offer = order(TokenOrderSide::Bid, 1000, 400);
        offer.escrow = 1998;
        let block_info = BlockInfo::default();
        let document = offer
            .new_document(&block_info)
            .expect("expected a document");
        assert_eq!(
            TokenOrder::from_document(&document, TokenOrderSide::Bid).expect("expected an order"),
            offer
        );
        assert_eq!(document.revision(), Some(1));

        offer.remaining = 100;
        let updated = offer
            .updated_document(&document, &block_info)
            .expect("expected a document");
        assert_eq!(updated.revision(), Some(2));
        assert_eq!(
            TokenOrder::from_document(&updated, TokenOrderSide::Bid)
                .expect("expected an order")
                .remaining,
            100
        );
    }
}
