//! End-to-end coverage for the `propertyConstraints` doctype keyword (protocol
//! version 14): a document type names rules its documents' properties must
//! meet, each a comparison of two integer expressions, of a string or an
//! identifier property with constants or with another property of its kind,
//! an `in` list of values, a `present` or `absent` test, or an `anyOf`,
//! `allOf` or `not` of such conditions. A create or replace that breaks one is
//! consensus-rejected with `DocumentPropertyConstraintViolatedError` (basic
//! code 10422), naming the rule and why, and leaves the stored document
//! untouched. A property the document leaves out counts as 0 in an operand,
//! or as its `ifAbsent` value.

use super::*;

mod property_constraints_tests {
    use super::*;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
    use crate::execution::validation::state_transition::tests::setup_identity_without_adding_it;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::document::PropertyConstraintViolation;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::document::DocumentV0Setters;
    use dpp::fee::Credits;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::prelude::{DataContract, Identifier, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;
    use std::collections::{BTreeMap, BTreeSet};

    /// The bytes repeated into the token ids `paidInAcceptedToken` lists, base58.
    const ACCEPTED_TOKENS: [u8; 2] = [7, 8];

    /// A mutable `offer` type whose rules, checked in name order, are:
    ///
    /// * `boostCapped`: `price * ifAbsent(boost, 1) <= 100000`
    /// * `boostPower`: `ifAbsent(boost, 1) ^ 20 >= 1`, which overflows for a large boost
    /// * `closedAtOnlyWhenClosed`: `closedAt` only on a closed or cancelled offer
    /// * `closedNeedsClosedAt`: a closed offer carries `closedAt`
    /// * `depositCoversOrder`: `(price + fee) * quantity <= deposit`
    /// * `discountBelowPrice`: `discount < price`, an absent discount counting as 0
    /// * `discountGivenAboveZero`: `discount` is absent or above 0
    /// * `discountOnlyWhileOpen`: a discount only on an open offer, a status left out
    ///   counting as open
    /// * `feeWaivedOnlyWithDiscount`: `!(fee == 0 && discount == 0)`
    /// * `feeWaivedOrAtLeastTen`: `fee == 0 || fee >= 10`
    /// * `paidInAcceptedToken`: a `paymentToken`, when given, is one of [`ACCEPTED_TOKENS`]
    /// * `perUnitDeposit`: `deposit / quantity >= 1`, which divides by zero for no quantity
    /// * `refundGoesToPayer`: a `refundTo`, when given, is the `payerId`
    /// * `settlesInAnotherCurrency`: a `settleIn` currency, when given, is not `currency`
    /// * `tieredFee`: `fee` is one of 0, 10, 25 or 50
    /// * `waivedFeeIsZero`: `waiveFee * fee == 0`, the boolean reading as 1 or 0
    fn offer_schema() -> Value {
        let accepted_tokens = ACCEPTED_TOKENS
            .map(|byte| Value::Text(Identifier::new([byte; 32]).to_string(Encoding::Base58)));
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "price": { "type": "integer", "minimum": 0, "maximum": 1000000, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "maximum": 100, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "discount": { "type": "integer", "minimum": 0, "maximum": 1000000, "position": 4 },
                "boost": { "type": "integer", "minimum": 0, "maximum": 100, "position": 5 },
                "waiveFee": { "type": "boolean", "position": 6 },
                "status": {
                    "type": "string",
                    "enum": ["open", "closed", "cancelled"],
                    "maxLength": 9,
                    "position": 7
                },
                "closedAt": { "type": "integer", "minimum": 0, "position": 8 },
                "currency": {
                    "type": "string",
                    "enum": ["USD", "EUR", "DASH"],
                    "maxLength": 4,
                    "position": 9
                },
                "settleIn": {
                    "type": "string",
                    "enum": ["USD", "EUR", "DASH"],
                    "maxLength": 4,
                    "position": 10
                },
                "payerId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 11
                },
                "refundTo": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 12
                },
                "paymentToken": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 13
                }
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "boostCapped": {
                    "lessThanOrEqual": [
                        { "multiply": ["price", { "ifAbsent": ["boost", 1] }] },
                        100000
                    ]
                },
                "boostPower": {
                    "greaterThanOrEqual": [{ "power": [{ "ifAbsent": ["boost", 1] }, 20] }, 1]
                },
                "closedAtOnlyWhenClosed": {
                    "anyOf": [
                        { "in": ["status", ["closed", "cancelled"]] },
                        { "absent": "closedAt" }
                    ]
                },
                "closedNeedsClosedAt": {
                    "anyOf": [
                        { "notEqual": ["status", { "const": "closed" }] },
                        { "present": "closedAt" }
                    ]
                },
                "depositCoversOrder": {
                    "lessThanOrEqual": [
                        { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
                        "deposit"
                    ]
                },
                "discountBelowPrice": { "lessThan": ["discount", "price"] },
                "discountGivenAboveZero": {
                    "anyOf": [{ "absent": "discount" }, { "greaterThan": ["discount", 0] }]
                },
                "discountOnlyWhileOpen": {
                    "anyOf": [
                        { "absent": "discount" },
                        { "equal": [{ "ifAbsent": ["status", "open"] }, { "const": "open" }] }
                    ]
                },
                "feeWaivedOnlyWithDiscount": {
                    "not": { "allOf": [{ "equal": ["fee", 0] }, { "equal": ["discount", 0] }] }
                },
                "feeWaivedOrAtLeastTen": {
                    "anyOf": [{ "equal": ["fee", 0] }, { "greaterThanOrEqual": ["fee", 10] }]
                },
                "paidInAcceptedToken": {
                    "anyOf": [
                        { "absent": "paymentToken" },
                        { "in": ["paymentToken", accepted_tokens] }
                    ]
                },
                "perUnitDeposit": {
                    "greaterThanOrEqual": [{ "divide": ["deposit", "quantity"] }, 1]
                },
                "refundGoesToPayer": {
                    "anyOf": [{ "absent": "refundTo" }, { "equal": ["refundTo", "payerId"] }]
                },
                "settlesInAnotherCurrency": {
                    "anyOf": [{ "absent": "settleIn" }, { "notEqual": ["settleIn", "currency"] }]
                },
                "tieredFee": { "in": ["fee", [0, 10, 25, 50]] },
                "waivedFeeIsZero": { "equal": [{ "multiply": ["waiveFee", "fee"] }, 0] }
            },
            "additionalProperties": false
        })
    }

    /// A mutable, transferable and purchasable `offer` type with the integers
    /// [`set_valid_offer`] fills and one rule, `sellerIsOwner`: a `sellerId`,
    /// when given, is the offer's owner, `$ownerId`, so a transfer or a purchase
    /// of an offer naming its seller is refused.
    fn owned_offer_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "transferable": 1,
            "tradeMode": 1,
            "properties": {
                "price": { "type": "integer", "minimum": 0, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "sellerId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 4
                }
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "sellerIsOwner": {
                    "anyOf": [{ "absent": "sellerId" }, { "equal": ["sellerId", "$ownerId"] }]
                }
            },
            "additionalProperties": false
        })
    }

    /// [`owned_offer_schema`] with a `meta` object holding a `tag` and an
    /// `inner` object, and one rule instead, `metaOrSeller`: the offer carries
    /// `meta`, or its owner is its seller.
    fn described_offer_schema() -> Value {
        let mut schema = owned_offer_schema();
        schema["properties"]["meta"] = platform_value!({
            "type": "object",
            "position": 5,
            "properties": {
                "tag": { "type": "string", "maxLength": 30, "position": 0 },
                "inner": {
                    "type": "object",
                    "position": 1,
                    "properties": {
                        "note": { "type": "string", "maxLength": 30, "position": 0 }
                    },
                    "additionalProperties": false
                }
            },
            "additionalProperties": false
        });
        schema["propertyConstraints"] = platform_value!({
            "metaOrSeller": {
                "anyOf": [{ "present": "meta" }, { "equal": ["sellerId", "$ownerId"] }]
            }
        });
        schema
    }

    /// An `offer` type with the integers [`set_valid_offer`] fills, a `title`, a
    /// typed array of `tags` and a byte array `signature`, and four rules on
    /// their sizes: `shortTitle` (at most 10 characters), `titleBytes` (at most
    /// 12 UTF-8 bytes), `tagsPerUnit` (no more tags than the quantity) and
    /// `signatureLength` (left out, or 64 or 65 bytes).
    fn sized_offer_schema() -> Value {
        platform_value!({
            "type": "object",
            "properties": {
                "price": { "type": "integer", "minimum": 0, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "title": { "type": "string", "maxLength": 40, "position": 4 },
                "tags": {
                    "type": "array",
                    "maxItems": 8,
                    "items": { "type": "string", "maxLength": 16 },
                    "position": 5
                },
                "signature": {
                    "type": "array",
                    "byteArray": true,
                    "maxItems": 65,
                    "position": 6
                }
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "shortTitle": { "lessThanOrEqual": [{ "length": "title" }, 10] },
                "titleBytes": { "lessThanOrEqual": [{ "byteLength": "title" }, 12] },
                "tagsPerUnit": { "lessThanOrEqual": [{ "count": "tags" }, "quantity"] },
                "signatureLength": { "in": [{ "count": "signature" }, [0, 64, 65]] }
            },
            "additionalProperties": false
        })
    }

    /// A mutable, transferable and purchasable `offer` type with the integers
    /// [`set_valid_offer`] fills and an `endsAt` time, recording the time of its
    /// creation, last update and last transfer and the block height of its
    /// creation, and five rules on them: `endsAfterCreation`
    /// (`endsAt > $createdAt`), `endsWithinAWeek` (`endsAt - $createdAt` at most
    /// a week), `listedAfterHeight10` (`$createdAtBlockHeight >= 10`),
    /// `updatedBeforeEnd` (`$updatedAt <= endsAt`) and `transferredBeforeEnd`
    /// (`$transferredAt <= endsAt`).
    fn timed_offer_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "transferable": 1,
            "tradeMode": 1,
            "properties": {
                "price": { "type": "integer", "minimum": 0, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "endsAt": { "type": "integer", "minimum": 0, "position": 4 }
            },
            "required": [
                "price",
                "fee",
                "quantity",
                "deposit",
                "endsAt",
                "$createdAt",
                "$updatedAt",
                "$transferredAt",
                "$createdAtBlockHeight"
            ],
            "propertyConstraints": {
                "endsAfterCreation": { "greaterThan": ["endsAt", "$createdAt"] },
                "endsWithinAWeek": {
                    "lessThanOrEqual": [{ "subtract": ["endsAt", "$createdAt"] }, WEEK_MS]
                },
                "listedAfterHeight10": { "greaterThanOrEqual": ["$createdAtBlockHeight", 10] },
                "updatedBeforeEnd": { "lessThanOrEqual": ["$updatedAt", "endsAt"] },
                "transferredBeforeEnd": { "lessThanOrEqual": ["$transferredAt", "endsAt"] }
            },
            "additionalProperties": false
        })
    }

    const DAY_MS: u64 = 86_400_000;
    const WEEK_MS: u64 = 7 * DAY_MS;
    /// The block time the timed tests start at.
    const NOW: u64 = 1_700_000_000_000;

    /// A block at `time_ms` and Platform `height`.
    fn at_block(time_ms: u64, height: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            height,
            core_height: 1000,
            ..Default::default()
        }
    }

    /// The fixture over [`timed_offer_schema`] with an offer created at [`NOW`],
    /// block 20, ending a day later.
    async fn timed_offer() -> OfferFixture {
        let mut fixture = OfferFixture::with_schema(timed_offer_schema());
        fixture.block_info = at_block(NOW, 20);
        assert_matches!(
            fixture
                .create(|document| document.set("endsAt", Value::U64(NOW + DAY_MS)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        fixture
    }

    /// A mutable, transferable `offer` type with the integers
    /// [`set_valid_offer`] fills and three typed arrays, of `labels`, of
    /// `members` and of `tiers`, with three rules looking among them:
    /// `notUsed` (no `"used"` label), `ownerIsMember` (the owner is a member,
    /// when members are listed) and `quantityListed` (the quantity is one of the
    /// tiers, when tiers are listed).
    fn listed_offer_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "transferable": 1,
            "properties": {
                "price": { "type": "integer", "minimum": 0, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "labels": {
                    "type": "array",
                    "maxItems": 4,
                    "items": { "type": "string", "maxLength": 10, "enum": ["new", "used", "sale"] },
                    "position": 4
                },
                "members": {
                    "type": "array",
                    "maxItems": 4,
                    "items": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier"
                    },
                    "position": 5
                },
                "tiers": {
                    "type": "array",
                    "maxItems": 4,
                    "items": { "type": "integer", "minimum": 0 },
                    "position": 6
                }
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "notUsed": { "not": { "contains": ["labels", { "const": "used" }] } },
                "ownerIsMember": {
                    "anyOf": [{ "absent": "members" }, { "contains": ["members", "$ownerId"] }]
                },
                "quantityListed": {
                    "anyOf": [{ "absent": "tiers" }, { "contains": ["tiers", "quantity"] }]
                }
            },
            "additionalProperties": false
        })
    }

    /// An `offer` type with the integers [`set_valid_offer`] fills, a `url`, a
    /// `path` and a `parentPath`, with three rules on prefixes and suffixes:
    /// `dashDomain` (a url ends with `.dash`), `secureUrl` (a url starts with
    /// `https://`) and `underParent` (a path starts with its parent's).
    fn linked_offer_schema() -> Value {
        platform_value!({
            "type": "object",
            "properties": {
                "price": { "type": "integer", "minimum": 0, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "url": { "type": "string", "maxLength": 100, "position": 4 },
                "path": { "type": "string", "maxLength": 100, "position": 5 },
                "parentPath": { "type": "string", "maxLength": 100, "position": 6 }
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "dashDomain": {
                    "anyOf": [{ "absent": "url" }, { "endsWith": ["url", { "const": ".dash" }] }]
                },
                "secureUrl": {
                    "anyOf": [{ "absent": "url" }, { "startsWith": ["url", { "const": "https://" }] }]
                },
                "underParent": {
                    "anyOf": [{ "absent": "parentPath" }, { "startsWith": ["path", "parentPath"] }]
                }
            },
            "additionalProperties": false
        })
    }

    /// An `offer` type with the integers [`set_valid_offer`] fills and a
    /// `discount`, with one rule per shorthand operator: `depositNearTotal`
    /// (`abs`: the deposit is within 5 of the order total), `discountNeedsPrice`
    /// (`ifThen`: a discount needs a price of 100 or more), `feeCapped` (`max`:
    /// the fee is at most 10 or a tenth of the price), `feeNotBanned` (`notIn`),
    /// `noZeroTerms` (`min`: price, fee and quantity are all above 0) and
    /// `quantityTiers` (`ifThenElse`: at most 10 at a price of 100 or more, at
    /// most 100 below it).
    fn shorthand_offer_schema() -> Value {
        platform_value!({
            "type": "object",
            "properties": {
                "price": { "type": "integer", "minimum": 0, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "discount": { "type": "integer", "minimum": 0, "position": 4 }
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "depositNearTotal": {
                    "lessThanOrEqual": [
                        {
                            "abs": {
                                "subtract": [
                                    "deposit",
                                    { "multiply": [{ "add": ["price", "fee"] }, "quantity"] }
                                ]
                            }
                        },
                        5
                    ]
                },
                "discountNeedsPrice": {
                    "ifThen": [
                        { "greaterThan": ["discount", 0] },
                        { "greaterThanOrEqual": ["price", 100] }
                    ]
                },
                "feeCapped": {
                    "lessThanOrEqual": ["fee", { "max": [10, { "divide": ["price", 10] }] }]
                },
                "feeNotBanned": { "notIn": ["fee", [7, 13]] },
                "noZeroTerms": {
                    "greaterThan": [{ "min": ["price", "fee", "quantity"] }, 0]
                },
                "quantityTiers": {
                    "ifThenElse": [
                        { "greaterThanOrEqual": ["price", 100] },
                        { "lessThanOrEqual": ["quantity", 10] },
                        { "lessThanOrEqual": ["quantity", 100] }
                    ]
                }
            },
            "additionalProperties": false
        })
    }

    /// An offer that meets every rule: (100 + 10) * 2 = 220.
    fn set_valid_offer(document: &mut Document) {
        document.set("price", Value::U64(100));
        document.set("fee", Value::U64(10));
        document.set("quantity", Value::U64(2));
        document.set("deposit", Value::U64(220));
    }

    /// One identity and one contract whose `offer` type declares the rules
    /// above. `create` writes a document, `replace` rewrites the last accepted
    /// one.
    struct OfferFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        contract: DataContract,
        /// The document as last accepted by the chain, once one was.
        document: Option<Document>,
        /// The identity contract nonce the next transition uses. Every
        /// processed transition consumes one, including the ones that fail
        /// with a paid consensus error.
        next_nonce: IdentityNonce,
        /// The block every transition is processed in: its time and heights are
        /// the ones a write records.
        block_info: BlockInfo,
    }

    impl OfferFixture {
        fn new() -> Self {
            Self::with_schema(offer_schema())
        }

        /// The fixture with its `offer` type declared by `schema`.
        fn with_schema(schema: Value) -> Self {
            Self::with_schemas(schema, [])
        }

        /// The fixture with its `offer` type declared by `schema`, beside the
        /// `others`, each by its name and schema.
        fn with_schemas<const N: usize>(schema: Value, others: [(&str, Value); N]) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 959, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema("offer", schema, true, &mut Vec::new(), platform_version)
                .expect("expected to add the offer document type");
            for (name, schema) in others {
                contract
                    .set_document_schema(name, schema, true, &mut Vec::new(), platform_version)
                    .expect("expected to add the document type");
            }
            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to apply the contract");

            Self {
                platform,
                identity,
                signer,
                key,
                contract,
                document: None,
                next_nonce: 1,
                block_info: BlockInfo::default(),
            }
        }

        fn process(&self, transition: &StateTransition) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let platform_state = self.platform.state.load();
            let serialized = transition
                .serialize_to_bytes()
                .expect("expected the transition to serialize");
            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &self.block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the state transition");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transaction");
            processing_result.into_execution_results().remove(0)
        }

        /// Creates a valid offer changed by `fill`. On success it becomes the
        /// fixture's document.
        async fn create(
            &mut self,
            fill: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            self.create_of("offer", |document| {
                set_valid_offer(document);
                fill(document);
            })
            .await
        }

        /// Creates a document of the type `document_type` changed by `fill`. On
        /// success an offer becomes the fixture's document.
        async fn create_of(
            &mut self,
            document_type: &str,
            fill: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let offer_type = self
                .contract
                .document_type_for_name(document_type)
                .expect("expected the document type");

            let mut rng = StdRng::seed_from_u64(434);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = offer_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random offer");
            document
                .set_id_for_creation(offer_type, &entropy.0, self.next_nonce, platform_version)
                .expect("expected to set the document id");
            fill(&mut document);

            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                offer_type,
                entropy.0,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            self.next_nonce += 1;

            let result = self.process(&transition);
            if document_type == "offer"
                && matches!(
                    result,
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                )
            {
                self.document = Some(document);
            }
            result
        }

        /// Replaces the stored offer with `mutate` applied to a copy of it and
        /// the revision bumped. On success the fixture's document becomes the
        /// accepted version.
        async fn replace(
            &mut self,
            mutate: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = self
                .document
                .clone()
                .expect("a document must have been created first");
            mutate(&mut replacement);
            replacement
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let offer_type = self
                    .contract
                    .document_type_for_name("offer")
                    .expect("expected the offer document type");
                BatchTransition::new_document_replacement_transition_from_document(
                    replacement.clone(),
                    offer_type,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the replace transition")
            };
            self.next_nonce += 1;

            let result = self.process(&transition);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                self.document = Some(replacement);
            }
            result
        }

        /// Transfers the stored offer to `recipient`. On success the fixture's
        /// document becomes the transferred version.
        async fn transfer(&mut self, recipient: Identifier) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut transferred = self
                .document
                .clone()
                .expect("a document must have been created first");
            transferred
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let offer_type = self
                    .contract
                    .document_type_for_name("offer")
                    .expect("expected the offer document type");
                BatchTransition::new_document_transfer_transition_from_document(
                    transferred.clone(),
                    offer_type,
                    recipient,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the transfer transition")
            };
            self.next_nonce += 1;

            let result = self.process(&transition);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                transferred.set_owner_id(recipient);
                self.document = Some(transferred);
            }
            result
        }

        /// Puts the stored offer up for sale at `price`, which must succeed.
        async fn set_price(&mut self, price: Credits) {
            assert_matches!(
                self.try_set_price(price).await,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "setting the price must succeed"
            );
        }

        /// Puts the stored offer up for sale at `price`. On success the fixture's
        /// document becomes the priced version.
        async fn try_set_price(&mut self, price: Credits) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut priced = self
                .document
                .clone()
                .expect("a document must have been created first");
            priced
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let offer_type = self
                    .contract
                    .document_type_for_name("offer")
                    .expect("expected the offer document type");
                BatchTransition::new_document_update_price_transition_from_document(
                    priced.clone(),
                    offer_type,
                    price,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the update price transition")
            };
            self.next_nonce += 1;

            let result = self.process(&transition);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                self.document = Some(priced);
            }
            result
        }

        /// A second funded identity on the fixture's platform.
        fn other_identity(&mut self, seed: u64) -> (Identity, SimpleSigner, IdentityPublicKey) {
            setup_identity(&mut self.platform, seed, dash_to_credits!(0.5))
        }

        /// `buyer` purchases the stored offer at `price` (its first transition,
        /// so nonce 1).
        async fn purchase_by(
            &mut self,
            buyer: &(Identity, SimpleSigner, IdentityPublicKey),
            price: Credits,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let (buyer_identity, buyer_signer, buyer_key) = buyer;
            let mut bought = self
                .document
                .clone()
                .expect("a document must have been created first");
            bought
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let offer_type = self
                    .contract
                    .document_type_for_name("offer")
                    .expect("expected the offer document type");
                BatchTransition::new_document_purchase_transition_from_document(
                    bought,
                    offer_type,
                    buyer_identity.id(),
                    price,
                    buyer_key,
                    1,
                    0,
                    None,
                    buyer_signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the purchase transition")
            };

            self.process(&transition)
        }

        fn stored_offers(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from offer",
                &self.contract,
                Some(&self.platform.config.drive),
                platform_version,
            )
            .expect("expected a document query");
            self.platform
                .drive
                .query_documents(query, None, false, None, None)
                .expect("expected a query result")
                .documents()
                .to_vec()
        }
    }

    /// An integer property of a stored document, whatever width it was read back at.
    fn integer(document: &Document, property: &str) -> Option<u64> {
        document
            .get(property)
            .and_then(|value| value.to_integer::<u64>().ok())
    }

    fn expect_violated(
        result: StateTransitionExecutionResult,
        expected_constraint: &str,
        expected_violation: PropertyConstraintViolation,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        let ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(error)) =
            error
        else {
            panic!("expected DocumentPropertyConstraintViolatedError, got {error:?}");
        };
        assert_eq!(error.document_type_name(), "offer");
        assert_eq!(error.constraint(), expected_constraint);
        assert_eq!(error.violation(), expected_violation);
        assert_eq!(ConsensusError::from(error).code(), 10422);
    }

    #[tokio::test]
    async fn should_accept_a_create_that_meets_every_rule() {
        let mut fixture = OfferFixture::new();

        assert_matches!(
            fixture.create(|_| {}).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(integer(&stored[0], "deposit"), Some(220));
    }

    #[tokio::test]
    async fn should_reject_a_create_whose_comparison_does_not_hold() {
        let mut fixture = OfferFixture::new();

        // (100 + 10) * 2 = 220 > 219
        let result = fixture
            .create(|document| document.set("deposit", Value::U64(219)))
            .await;

        expect_violated(
            result,
            "depositCoversOrder",
            PropertyConstraintViolation::NotMet,
        );
        assert!(
            fixture.stored_offers().is_empty(),
            "the refused document must not be stored"
        );
    }

    /// `discount` is absent, so it counts as 0, and `0 < 0` does not hold.
    #[tokio::test]
    async fn should_count_an_absent_property_as_zero() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| {
                document.set("price", Value::U64(0));
                document.set("fee", Value::U64(0));
            })
            .await;

        expect_violated(
            result,
            "discountBelowPrice",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());
    }

    /// `boost` is absent, so `ifAbsent` makes it 1: `200000 * 1` is above the cap,
    /// where a boost of 0 would have met it. A boost the document carries is used.
    #[tokio::test]
    async fn should_take_the_if_absent_value_for_an_absent_property() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| {
                document.set("price", Value::U64(200000));
                document.set("deposit", Value::U64(500000));
            })
            .await;
        expect_violated(result, "boostCapped", PropertyConstraintViolation::NotMet);

        // 50000 * 3 = 150000 is above the cap as well
        let result = fixture
            .create(|document| {
                document.set("price", Value::U64(50000));
                document.set("deposit", Value::U64(200000));
                document.set("boost", Value::U64(3));
            })
            .await;
        expect_violated(result, "boostCapped", PropertyConstraintViolation::NotMet);

        // 50000 * 2 = 100000 meets it
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("price", Value::U64(50000));
                    document.set("deposit", Value::U64(200000));
                    document.set("boost", Value::U64(2));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// A zero quantity divides the deposit by zero, and a boost of 100 raised to the
    /// power 20 does not fit 128 bits: both refuse the document instead of wrapping.
    #[tokio::test]
    async fn should_reject_a_create_that_divides_by_zero_or_overflows() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| document.set("quantity", Value::U64(0)))
            .await;
        expect_violated(
            result,
            "perUnitDeposit",
            PropertyConstraintViolation::DivisionByZero,
        );

        let result = fixture
            .create(|document| document.set("boost", Value::U64(100)))
            .await;
        expect_violated(result, "boostPower", PropertyConstraintViolation::Overflow);

        assert!(fixture.stored_offers().is_empty());
    }

    /// A fee of 5 is neither waived nor at least 10, and a waived fee needs a
    /// discount; a waived fee on a discounted offer meets both rules.
    #[tokio::test]
    async fn should_judge_any_of_all_of_and_not() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| document.set("fee", Value::U64(5)))
            .await;
        expect_violated(
            result,
            "feeWaivedOrAtLeastTen",
            PropertyConstraintViolation::NotMet,
        );

        let result = fixture
            .create(|document| document.set("fee", Value::U64(0)))
            .await;
        expect_violated(
            result,
            "feeWaivedOnlyWithDiscount",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        // (100 + 0) * 2 = 200 <= 220, and 10 < 100
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("fee", Value::U64(0));
                    document.set("discount", Value::U64(10));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// A discount may be left out, but one the offer gives must be above 0: only
    /// a presence test tells the two apart, since an operand reads a discount left
    /// out as 0.
    #[tokio::test]
    async fn should_tell_a_property_left_out_from_one_set_to_zero() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| document.set("discount", Value::U64(0)))
            .await;
        expect_violated(
            result,
            "discountGivenAboveZero",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture.create(|_| {}).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture
                .create(|document| document.set("discount", Value::U64(10)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// A fee of 20 is not one of the tiers; 25 is. (100 + 25) * 2 = 250 <= 300.
    #[tokio::test]
    async fn should_judge_an_in_against_its_listed_values() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| {
                document.set("fee", Value::U64(20));
                document.set("deposit", Value::U64(300));
            })
            .await;
        expect_violated(result, "tieredFee", PropertyConstraintViolation::NotMet);
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| {
                    document.set("fee", Value::U64(25));
                    document.set("deposit", Value::U64(300));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// A boolean reads as 1 for true and 0 for false: a waived fee must be 0, and
    /// an offer that does not waive it, or leaves the flag out, may charge one.
    #[tokio::test]
    async fn should_read_a_boolean_property_as_one_or_zero() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| document.set("waiveFee", Value::Bool(true)))
            .await;
        expect_violated(
            result,
            "waivedFeeIsZero",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        // A waived fee of 0 needs a discount (`feeWaivedOnlyWithDiscount`)
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("waiveFee", Value::Bool(true));
                    document.set("fee", Value::U64(0));
                    document.set("discount", Value::U64(10));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture
                .create(|document| document.set("waiveFee", Value::Bool(false)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// A string property is compared with constants: a closed offer needs
    /// `closedAt`, and only a closed or cancelled one may carry it.
    #[tokio::test]
    async fn should_compare_a_string_property_with_constants() {
        let mut fixture = OfferFixture::new();
        let status = |value: &str| Value::Text(value.to_string());

        let result = fixture
            .create(|document| document.set("status", status("closed")))
            .await;
        expect_violated(
            result,
            "closedNeedsClosedAt",
            PropertyConstraintViolation::NotMet,
        );

        let result = fixture
            .create(|document| {
                document.set("status", status("open"));
                document.set("closedAt", Value::U64(1000));
            })
            .await;
        expect_violated(
            result,
            "closedAtOnlyWhenClosed",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        for (value, closed_at) in [
            ("closed", Some(1000)),
            ("cancelled", Some(1000)),
            ("open", None),
        ] {
            assert_matches!(
                fixture
                    .create(|document| {
                        document.set("status", status(value));
                        if let Some(closed_at) = closed_at {
                            document.set("closedAt", Value::U64(closed_at));
                        }
                    })
                    .await,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "{value}"
            );
        }
        assert_eq!(fixture.stored_offers().len(), 3);
    }

    /// Two string properties compare their strings: an offer may not settle in
    /// the currency it is priced in. A currency it leaves out equals none.
    #[tokio::test]
    async fn should_compare_two_string_properties() {
        let mut fixture = OfferFixture::new();
        let currency = |value: &str| Value::Text(value.to_string());

        let result = fixture
            .create(|document| {
                document.set("currency", currency("USD"));
                document.set("settleIn", currency("USD"));
            })
            .await;
        expect_violated(
            result,
            "settlesInAnotherCurrency",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| {
                    document.set("currency", currency("USD"));
                    document.set("settleIn", currency("DASH"));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // No price currency: the settlement currency differs from it
        assert_matches!(
            fixture
                .create(|document| document.set("settleIn", currency("EUR")))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// A string default stands in for a status the offer leaves out: a discount
    /// is allowed with no status, as on an open offer, but not on a closed one.
    #[tokio::test]
    async fn should_read_a_string_default_for_a_property_left_out() {
        let mut fixture = OfferFixture::new();
        let status = |value: &str| Value::Text(value.to_string());

        let result = fixture
            .create(|document| {
                document.set("discount", Value::U64(10));
                document.set("status", status("closed"));
                document.set("closedAt", Value::U64(1000));
            })
            .await;
        expect_violated(
            result,
            "discountOnlyWhileOpen",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| document.set("discount", Value::U64(10)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("discount", Value::U64(10));
                    document.set("status", status("open"));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// Identifier properties compare with the base58 identifiers an `in` lists
    /// and with each other: a payment token must be an accepted one, and a
    /// refund must go to the payer.
    #[tokio::test]
    async fn should_compare_identifier_properties() {
        let mut fixture = OfferFixture::new();
        let identifier = |byte: u8| Value::Identifier([byte; 32]);

        let result = fixture
            .create(|document| document.set("paymentToken", identifier(9)))
            .await;
        expect_violated(
            result,
            "paidInAcceptedToken",
            PropertyConstraintViolation::NotMet,
        );

        let result = fixture
            .create(|document| {
                document.set("payerId", identifier(1));
                document.set("refundTo", identifier(2));
            })
            .await;
        expect_violated(
            result,
            "refundGoesToPayer",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| {
                    document.set("paymentToken", identifier(ACCEPTED_TOKENS[1]));
                    document.set("payerId", identifier(1));
                    document.set("refundTo", identifier(1));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// A rule reading `$ownerId` holds the offer's seller to its owner: on a
    /// create, and on a transfer or a purchase, which change the owner.
    #[tokio::test]
    async fn should_compare_the_owner_on_create_transfer_and_purchase() {
        let mut fixture = OfferFixture::with_schema(owned_offer_schema());
        let owner = fixture.identity.id();

        let result = fixture
            .create(|document| document.set("sellerId", Value::Identifier([4; 32])))
            .await;
        expect_violated(result, "sellerIsOwner", PropertyConstraintViolation::NotMet);

        assert_matches!(
            fixture
                .create(|document| document.set("sellerId", Value::Identifier(owner.to_buffer())))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Transferred, the offer would name a seller that no longer owns it
        let (recipient, _, _) = fixture.other_identity(961);
        let result = fixture.transfer(recipient.id()).await;
        expect_violated(result, "sellerIsOwner", PropertyConstraintViolation::NotMet);

        // Bought, likewise
        fixture.set_price(1000).await;
        let buyer = fixture.other_identity(962);
        let result = fixture.purchase_by(&buyer, 1000).await;
        expect_violated(result, "sellerIsOwner", PropertyConstraintViolation::NotMet);

        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].owner_id(),
            owner,
            "the refused actions leave the owner"
        );
    }

    /// An offer naming no seller moves freely: the rule reading `$ownerId` holds
    /// whoever owns it.
    #[tokio::test]
    async fn should_transfer_an_offer_the_owner_rules_allow() {
        let mut fixture = OfferFixture::with_schema(owned_offer_schema());
        assert_matches!(
            fixture.create(|_| {}).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let (recipient, _, _) = fixture.other_identity(963);
        assert_matches!(
            fixture.transfer(recipient.id()).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers()[0].owner_id(), recipient.id());
    }

    /// A stored offer does not keep an object none of whose members it holds,
    /// so `present` reads `meta: {}` (or `{ "inner": {} }`) as absent on a
    /// create, as a transfer later reads the stored offer: a create by an owner
    /// who is not the seller is refused, and so is the transfer of an offer
    /// stored with `meta: {}` to a recipient who is not the seller. A `meta`
    /// holding a member is present on both.
    #[tokio::test]
    async fn should_judge_an_object_without_members_absent_on_create_and_transfer() {
        let mut fixture = OfferFixture::with_schema(described_offer_schema());
        let owner = fixture.identity.id();
        let seller = Value::Identifier([4; 32]);

        for meta in [platform_value!({}), platform_value!({ "inner": {} })] {
            let result = fixture
                .create(|document| {
                    document.set("sellerId", seller.clone());
                    document.set("meta", meta.clone());
                })
                .await;
            expect_violated(result, "metaOrSeller", PropertyConstraintViolation::NotMet);
        }
        assert!(fixture.stored_offers().is_empty());

        // Its owner is its seller, so the offer is stored, without `meta`
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("sellerId", Value::Identifier(owner.to_buffer()));
                    document.set("meta", platform_value!({}));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers()[0].get("meta"), None);

        // Transferred, it is the offer the creates above were refused for
        let (recipient, _, _) = fixture.other_identity(965);
        let result = fixture.transfer(recipient.id()).await;
        expect_violated(result, "metaOrSeller", PropertyConstraintViolation::NotMet);
        assert_eq!(fixture.stored_offers()[0].owner_id(), owner);

        assert_matches!(
            fixture
                .create(|document| {
                    document.set("sellerId", seller.clone());
                    document.set("meta", platform_value!({ "tag": "x" }));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture.transfer(recipient.id()).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// A `sellerId` that declares `refersTo` an identity is compared with
    /// `$ownerId` as any identifier property is: the contract registers, a
    /// create naming another existing identity as seller is refused, one naming
    /// the owner is accepted, and a transfer is refused.
    #[tokio::test]
    async fn should_compare_an_identifier_property_that_declares_refers_to() {
        let mut schema = owned_offer_schema();
        schema["properties"]["sellerId"]["refersTo"] = platform_value!({ "type": "identity" });
        let mut fixture = OfferFixture::with_schema(schema);
        let owner = fixture.identity.id();
        let (other, _, _) = fixture.other_identity(964);

        let result = fixture
            .create(|document| document.set("sellerId", Value::Identifier(other.id().to_buffer())))
            .await;
        expect_violated(result, "sellerIsOwner", PropertyConstraintViolation::NotMet);
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| document.set("sellerId", Value::Identifier(owner.to_buffer())))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture.transfer(other.id()).await;
        expect_violated(result, "sellerIsOwner", PropertyConstraintViolation::NotMet);
        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].owner_id(), owner);
    }

    /// Identifier properties that declare `refersTo` an identity are compared
    /// with a const, with the identifiers an `in` lists and with each other on
    /// a create, as plain ones are. Every identity the rules name exists, so
    /// only a rule refuses a create and the accepted one meets its references.
    #[tokio::test]
    async fn should_judge_const_in_and_pair_rules_over_refers_to_identifiers() {
        let seeds = [965, 966, 967, 968];
        let [payer, token_a, token_b, banned] =
            seeds.map(|seed| setup_identity_without_adding_it(seed, 0).0.id());
        let base58 = |id: Identifier| Value::Text(id.to_string(Encoding::Base58));
        let referring = |position: u32| {
            platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "refersTo": { "type": "identity" },
                "position": position
            })
        };
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "price": { "type": "integer", "minimum": 0, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "payerId": referring(4),
                "refundTo": referring(5),
                "paymentToken": referring(6)
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "paidInAcceptedToken": {
                    "anyOf": [
                        { "absent": "paymentToken" },
                        { "in": ["paymentToken", [base58(token_a), base58(token_b)]] }
                    ]
                },
                "payerNotBanned": {
                    "anyOf": [
                        { "absent": "payerId" },
                        { "notEqual": ["payerId", { "const": base58(banned) }] }
                    ]
                },
                "refundGoesToPayer": {
                    "anyOf": [{ "absent": "refundTo" }, { "equal": ["refundTo", "payerId"] }]
                }
            },
            "additionalProperties": false
        });
        let mut fixture = OfferFixture::with_schema(schema);
        for seed in seeds {
            fixture.other_identity(seed);
        }
        let identifier = |id: Identifier| Value::Identifier(id.to_buffer());

        let result = fixture
            .create(|document| document.set("paymentToken", identifier(banned)))
            .await;
        expect_violated(
            result,
            "paidInAcceptedToken",
            PropertyConstraintViolation::NotMet,
        );

        let result = fixture
            .create(|document| document.set("payerId", identifier(banned)))
            .await;
        expect_violated(
            result,
            "payerNotBanned",
            PropertyConstraintViolation::NotMet,
        );

        let result = fixture
            .create(|document| {
                document.set("payerId", identifier(payer));
                document.set("refundTo", identifier(token_a));
            })
            .await;
        expect_violated(
            result,
            "refundGoesToPayer",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| {
                    document.set("paymentToken", identifier(token_b));
                    document.set("payerId", identifier(payer));
                    document.set("refundTo", identifier(payer));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    #[tokio::test]
    async fn should_judge_a_replace_against_the_rules() {
        let mut fixture = OfferFixture::new();
        assert_matches!(
            fixture.create(|_| {}).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Doubling the quantity without the deposit breaks the rule
        let result = fixture
            .replace(|document| document.set("quantity", Value::U64(4)))
            .await;
        expect_violated(
            result,
            "depositCoversOrder",
            PropertyConstraintViolation::NotMet,
        );
        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            integer(&stored[0], "quantity"),
            Some(2),
            "the refused replace must leave the stored document untouched"
        );

        // Doubling both meets it
        assert_matches!(
            fixture
                .replace(|document| {
                    document.set("quantity", Value::U64(4));
                    document.set("deposit", Value::U64(440));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(integer(&fixture.stored_offers()[0], "quantity"), Some(4));
    }

    /// A replace carries the whole document, so a property it leaves out is absent
    /// from the rules' point of view even though the stored document had it: it counts
    /// as 0, or as its `ifAbsent` value, never as the stored value. Each replace below
    /// is accepted only that way: the stored `discount` (50) would break
    /// `discountBelowPrice` at a price of 40, and the stored `boost` (2) would break
    /// `boostCapped` at a price of 60000.
    #[tokio::test]
    async fn should_judge_a_replace_that_leaves_out_an_operand_by_its_absent_value() {
        let mut fixture = OfferFixture::new();
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("discount", Value::U64(50));
                    document.set("boost", Value::U64(2));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // discount absent, 0 < 40; (40 + 10) * 2 = 100 <= 220
        assert_matches!(
            fixture
                .replace(|document| {
                    document.remove("discount");
                    document.set("price", Value::U64(40));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // boost absent, 60000 * 1 <= 100000; (60000 + 10) * 2 = 120020
        assert_matches!(
            fixture
                .replace(|document| {
                    document.remove("boost");
                    document.set("price", Value::U64(60000));
                    document.set("deposit", Value::U64(120020));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(integer(&stored[0], "price"), Some(60000));
        assert_eq!(stored[0].get("discount"), None);
        assert_eq!(stored[0].get("boost"), None);
    }

    /// The replace structure dispatcher on both sides of the gate: structure
    /// generation 0 reaches the `propertyConstraints` check through
    /// `DataContract::validate_document_properties` 0, extended in place, so at
    /// protocol version 13 it must still accept the action (no type parsed there
    /// carries a rule and the dpp gate is `None`), and at 14 refuse it. The
    /// action is built by hand the way the transformer would build it, against
    /// the contract as Drive hands it back.
    #[test]
    fn should_not_judge_property_constraints_on_replace_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = OfferFixture::new();
        let owner_id = fixture.identity.id();

        let (_, contract_fetch_info) = fixture
            .platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                fixture.contract.id().to_buffer(),
                None,
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract");
        let contract_fetch_info = contract_fetch_info.expect("the contract is in state");

        let action = DocumentReplaceTransitionAction::V0(DocumentReplaceTransitionActionV0 {
            base: DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                id: Identifier::from([0xAA; 32]),
                identity_contract_nonce: 1,
                document_type_name: "offer".to_string(),
                data_contract: contract_fetch_info,
                token_cost: None,
                shielded_token_payment: None,
                gas_fees_paid_by: GasFeesPaidBy::default(),
                contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                declared_action_fee: None,
            }),
            revision: 2,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            // (100 + 10) * 2 = 220 > 1
            data: BTreeMap::from([
                ("price".to_string(), Value::U64(100)),
                ("fee".to_string(), Value::U64(10)),
                ("quantity".to_string(), Value::U64(2)),
                ("deposit".to_string(), Value::U64(1)),
            ]),
            changed_data_fields: BTreeSet::new(),
            removed_identifier_fields: BTreeMap::new(),
            stored_changed_values: BTreeMap::new(),
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
            property_constraint_aggregates: Default::default(),
        });

        let before = action
            .validate_structure(
                owner_id,
                PlatformVersion::get(13).expect("platform version 13 should exist"),
            )
            .expect("structure validation should run");
        assert!(
            before.is_valid(),
            "structure generation 0 must not judge propertyConstraints: {:?}",
            before.errors
        );

        let at = action
            .validate_structure(owner_id, platform_version)
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(e))]
                if e.constraint() == "depositCoversOrder"
                    && e.violation() == PropertyConstraintViolation::NotMet
        );
    }

    /// Sizes read by real creates: a title too long in characters, one short
    /// enough in characters but too long in bytes, more tags than the quantity
    /// and a signature of the wrong length are each refused with the rule they
    /// break, and an offer meeting all four is stored.
    #[tokio::test]
    async fn should_judge_the_sizes_of_strings_arrays_and_byte_arrays() {
        let mut fixture = OfferFixture::with_schema(sized_offer_schema());
        let tags = |count: usize| Value::Array(vec![Value::Text("tag".to_string()); count]);

        // 12 characters
        let result = fixture
            .create(|document| document.set("title", Value::from("a long title")))
            .await;
        expect_violated(result, "shortTitle", PropertyConstraintViolation::NotMet);

        // 8 characters, 16 bytes
        let result = fixture
            .create(|document| document.set("title", Value::from("éééééééé")))
            .await;
        expect_violated(result, "titleBytes", PropertyConstraintViolation::NotMet);

        // 3 tags for a quantity of 2
        let result = fixture
            .create(|document| document.set("tags", tags(3)))
            .await;
        expect_violated(result, "tagsPerUnit", PropertyConstraintViolation::NotMet);

        // 10 bytes
        let result = fixture
            .create(|document| document.set("signature", Value::Bytes(vec![7; 10])))
            .await;
        expect_violated(
            result,
            "signatureLength",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        // 4 characters in 5 bytes, 2 tags, a 64-byte signature
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("title", Value::from("Café"));
                    document.set("tags", tags(2));
                    document.set("signature", Value::Bytes(vec![7; 64]));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// The times and heights a create records are its block's: an offer must end
    /// after its creation and within a week of it, and be listed from block 10 on.
    #[tokio::test]
    async fn should_judge_a_create_by_the_time_and_height_of_its_block() {
        let mut fixture = OfferFixture::with_schema(timed_offer_schema());
        fixture.block_info = at_block(NOW, 20);

        let result = fixture
            .create(|document| document.set("endsAt", Value::U64(NOW)))
            .await;
        expect_violated(
            result,
            "endsAfterCreation",
            PropertyConstraintViolation::NotMet,
        );

        let result = fixture
            .create(|document| document.set("endsAt", Value::U64(NOW + 8 * DAY_MS)))
            .await;
        expect_violated(
            result,
            "endsWithinAWeek",
            PropertyConstraintViolation::NotMet,
        );

        fixture.block_info = at_block(NOW, 5);
        let result = fixture
            .create(|document| document.set("endsAt", Value::U64(NOW + DAY_MS)))
            .await;
        expect_violated(
            result,
            "listedAfterHeight10",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        fixture.block_info = at_block(NOW, 20);
        assert_matches!(
            fixture
                .create(|document| document.set("endsAt", Value::U64(NOW + DAY_MS)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].created_at(), Some(NOW));
    }

    /// A replace keeps the creation time and records its own update time: moving
    /// the end is measured from the stored `$createdAt`, and a replace after the
    /// end breaks `updatedBeforeEnd`. A price update, which records an update
    /// time too, is judged the same way.
    #[tokio::test]
    async fn should_judge_a_replace_and_a_price_update_by_the_update_time() {
        let mut fixture = timed_offer().await;

        // Three days on, the end moves to six days after creation
        fixture.block_info = at_block(NOW + 3 * DAY_MS, 30);
        assert_matches!(
            fixture
                .replace(|document| document.set("endsAt", Value::U64(NOW + 6 * DAY_MS)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // Nine days after creation is more than a week, whenever the replace happens
        let result = fixture
            .replace(|document| document.set("endsAt", Value::U64(NOW + 9 * DAY_MS)))
            .await;
        expect_violated(
            result,
            "endsWithinAWeek",
            PropertyConstraintViolation::NotMet,
        );

        // After the end, neither a replace nor a price update is accepted
        fixture.block_info = at_block(NOW + 7 * DAY_MS, 40);
        let result = fixture
            .replace(|document| document.set("fee", Value::U64(20)))
            .await;
        expect_violated(
            result,
            "updatedBeforeEnd",
            PropertyConstraintViolation::NotMet,
        );
        let result = fixture.try_set_price(1000).await;
        expect_violated(
            result,
            "updatedBeforeEnd",
            PropertyConstraintViolation::NotMet,
        );

        // Before it, the price update is accepted
        fixture.block_info = at_block(NOW + 5 * DAY_MS, 40);
        fixture.set_price(1000).await;
    }

    /// A transfer and a purchase record the transfer's time: after the end each
    /// breaks `transferredBeforeEnd`, while the rules reading the creation and
    /// update times are not judged again.
    #[tokio::test]
    async fn should_judge_a_transfer_and_a_purchase_by_the_transfer_time() {
        let mut fixture = timed_offer().await;
        let (recipient, _, _) = fixture.other_identity(7);

        fixture.block_info = at_block(NOW + 2 * DAY_MS, 30);
        let result = fixture.transfer(recipient.id()).await;
        expect_violated(
            result,
            "transferredBeforeEnd",
            PropertyConstraintViolation::NotMet,
        );

        fixture.block_info = at_block(NOW + DAY_MS / 2, 30);
        fixture.set_price(1000).await;
        let buyer = fixture.other_identity(8);
        fixture.block_info = at_block(NOW + 2 * DAY_MS, 40);
        let result = fixture.purchase_by(&buyer, 1000).await;
        expect_violated(
            result,
            "transferredBeforeEnd",
            PropertyConstraintViolation::NotMet,
        );

        fixture.block_info = at_block(NOW + DAY_MS / 2, 40);
        assert_matches!(
            fixture.transfer(recipient.id()).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// `contains` read by real writes: a `"used"` label, an owner missing from
    /// the members and a quantity missing from the tiers are each refused with
    /// the rule they break; a transfer, which changes the owner, is judged
    /// against `ownerIsMember` and refused to a non-member, accepted to a member.
    #[tokio::test]
    async fn should_judge_contains_on_create_and_transfer() {
        let mut fixture = OfferFixture::with_schema(listed_offer_schema());
        let (member, _, _) = fixture.other_identity(7);
        let (outsider, _, _) = fixture.other_identity(8);
        let owner = fixture.identity.id();
        let labels = |values: &[&str]| {
            Value::Array(values.iter().map(|value| Value::from(*value)).collect())
        };
        let members = |ids: &[Identifier]| {
            Value::Array(
                ids.iter()
                    .map(|id| Value::Identifier(id.to_buffer()))
                    .collect(),
            )
        };

        let result = fixture
            .create(|document| document.set("labels", labels(&["new", "used"])))
            .await;
        expect_violated(result, "notUsed", PropertyConstraintViolation::NotMet);

        let result = fixture
            .create(|document| document.set("members", members(&[member.id()])))
            .await;
        expect_violated(result, "ownerIsMember", PropertyConstraintViolation::NotMet);

        // The quantity is 2
        let result = fixture
            .create(|document| {
                document.set("tiers", Value::Array(vec![Value::U64(1), Value::U64(5)]))
            })
            .await;
        expect_violated(
            result,
            "quantityListed",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| {
                    document.set("labels", labels(&["new", "sale"]));
                    document.set("members", members(&[owner, member.id()]));
                    document.set("tiers", Value::Array(vec![Value::U64(2), Value::U64(10)]));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture.transfer(outsider.id()).await;
        expect_violated(result, "ownerIsMember", PropertyConstraintViolation::NotMet);
        assert_matches!(
            fixture.transfer(member.id()).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// `startsWith` and `endsWith` read by real creates: a url on another domain,
    /// one without https and a path outside its parent's are each refused with
    /// the rule they break, and an offer meeting all three is stored.
    #[tokio::test]
    async fn should_judge_prefixes_and_suffixes_on_create() {
        let mut fixture = OfferFixture::with_schema(linked_offer_schema());

        let result = fixture
            .create(|document| document.set("url", Value::from("https://shop.com")))
            .await;
        expect_violated(result, "dashDomain", PropertyConstraintViolation::NotMet);

        let result = fixture
            .create(|document| document.set("url", Value::from("http://shop.dash")))
            .await;
        expect_violated(result, "secureUrl", PropertyConstraintViolation::NotMet);

        let result = fixture
            .create(|document| {
                document.set("path", Value::from("a/c"));
                document.set("parentPath", Value::from("a/b"));
            })
            .await;
        expect_violated(result, "underParent", PropertyConstraintViolation::NotMet);
        assert!(fixture.stored_offers().is_empty());

        assert_matches!(
            fixture
                .create(|document| {
                    document.set("url", Value::from("https://shop.dash"));
                    document.set("path", Value::from("a/b/c"));
                    document.set("parentPath", Value::from("a/b"));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// The shorthand operators read by real creates: each of seven offers breaks
    /// exactly one rule and is refused with it, and the valid offers, one down
    /// each branch of the `ifThenElse`, are stored.
    #[tokio::test]
    async fn should_judge_min_max_abs_if_then_and_not_in_on_create() {
        let mut fixture = OfferFixture::with_schema(shorthand_offer_schema());
        let set = |document: &mut Document, entries: &[(&str, u64)]| {
            for (property, value) in entries {
                document.set(property, Value::U64(*value));
            }
        };

        // The total is 220: a deposit of 230 is 10 away
        let result = fixture
            .create(|document| set(document, &[("deposit", 230)]))
            .await;
        expect_violated(
            result,
            "depositNearTotal",
            PropertyConstraintViolation::NotMet,
        );

        // A discount on a price of 50 (total and deposit 120)
        let result = fixture
            .create(|document| {
                set(
                    document,
                    &[("price", 50), ("deposit", 120), ("discount", 5)],
                )
            })
            .await;
        expect_violated(
            result,
            "discountNeedsPrice",
            PropertyConstraintViolation::NotMet,
        );

        // A fee of 11 on a price of 100, above max(10, 10) (total 222)
        let result = fixture
            .create(|document| set(document, &[("fee", 11), ("deposit", 222)]))
            .await;
        expect_violated(result, "feeCapped", PropertyConstraintViolation::NotMet);

        // A banned fee of 7 (total and deposit 214)
        let result = fixture
            .create(|document| set(document, &[("fee", 7), ("deposit", 214)]))
            .await;
        expect_violated(result, "feeNotBanned", PropertyConstraintViolation::NotMet);

        // A quantity of 0 (total and deposit 0)
        let result = fixture
            .create(|document| set(document, &[("quantity", 0), ("deposit", 0)]))
            .await;
        expect_violated(result, "noZeroTerms", PropertyConstraintViolation::NotMet);

        // 11 at a price of 100, above the then branch's 10 (total and deposit 1210)
        let result = fixture
            .create(|document| set(document, &[("quantity", 11), ("deposit", 1210)]))
            .await;
        expect_violated(result, "quantityTiers", PropertyConstraintViolation::NotMet);

        // 101 at a price of 50, above the else branch's 100 (total and deposit 6060)
        let result = fixture
            .create(|document| {
                set(
                    document,
                    &[("price", 50), ("quantity", 101), ("deposit", 6060)],
                )
            })
            .await;
        expect_violated(result, "quantityTiers", PropertyConstraintViolation::NotMet);
        assert!(fixture.stored_offers().is_empty());

        // (100 + 10) * 2 = 220, no discount, a fee of 10
        assert_matches!(
            fixture.create(|_| {}).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // 50 at a price of 50, within the else branch's 100 (total and deposit 3000)
        assert_matches!(
            fixture
                .create(|document| {
                    set(
                        document,
                        &[("price", 50), ("quantity", 50), ("deposit", 3000)],
                    )
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// A mutable, transferable and purchasable `offer` type with the integers
    /// [`set_valid_offer`] fills (a price of at most 10^9, which a sum tree
    /// takes) and a required `category`, whose trees keep the count of each
    /// owner's offers (`byOwner`), of each owner's offers in each category
    /// (`byOwnerCategory`) and the total price of each category (`byCategory`),
    /// declaring `rules`.
    fn counted_offer_schema(rules: Value) -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "transferable": 1,
            "tradeMode": 1,
            "properties": {
                "price": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "category": { "type": "integer", "minimum": 0, "maximum": 100, "position": 4 }
            },
            "required": ["price", "fee", "quantity", "deposit", "category"],
            "indices": [
                {
                    "name": "byOwner",
                    "properties": [{ "$ownerId": "asc" }],
                    "countable": "countable"
                },
                {
                    "name": "byOwnerCategory",
                    "properties": [{ "$ownerId": "asc" }, { "category": "asc" }],
                    "countable": "countable"
                },
                {
                    "name": "byCategory",
                    "properties": [{ "category": "asc" }],
                    "summable": "price"
                }
            ],
            "propertyConstraints": rules,
            "additionalProperties": false
        })
    }

    /// Sets an offer's `category` and `price`.
    fn priced_in(category: u64, price: u64) -> impl FnOnce(&mut Document) {
        move |document: &mut Document| {
            document.set("category", Value::U64(category));
            document.set("price", Value::U64(price));
        }
    }

    /// `countOf` over the writer's own type by `$ownerId`: an identity owns at
    /// most two offers. The total is the one the tree keeps once the write is
    /// done, so a replace of one of the two, which leaves the count as it was,
    /// is judged by 2.
    #[tokio::test]
    async fn should_cap_the_offers_of_each_owner_on_create_and_replace() {
        let mut fixture = OfferFixture::with_schema(counted_offer_schema(platform_value!({
            "atMostTwoPerOwner": {
                "lessThanOrEqual": [{ "countOf": ["offer", { "$ownerId": "$ownerId" }] }, 2]
            }
        })));
        for _ in 0..2 {
            assert_matches!(
                fixture.create(priced_in(1, 100)).await,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }
        let result = fixture.create(priced_in(1, 100)).await;
        expect_violated(
            result,
            "atMostTwoPerOwner",
            PropertyConstraintViolation::NotMet,
        );

        assert_matches!(
            fixture.replace(priced_in(2, 150)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// `sumOf` over the offers of one category: their prices total at most 250.
    /// A create adds its price; a replace takes out the price it stored and adds
    /// the new one, and one moving the offer to another category takes its
    /// price out of the first.
    #[tokio::test]
    async fn should_total_the_prices_of_a_category_on_create_and_replace() {
        let mut fixture = OfferFixture::with_schema(counted_offer_schema(platform_value!({
            "categoryBudget": {
                "lessThanOrEqual": [
                    { "sumOf": ["offer", "price", { "category": "category" }] },
                    250
                ]
            }
        })));
        for price in [100, 100] {
            assert_matches!(
                fixture.create(priced_in(1, price)).await,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }
        // 200 + 60 is above 250
        let result = fixture.create(priced_in(1, 60)).await;
        expect_violated(
            result,
            "categoryBudget",
            PropertyConstraintViolation::NotMet,
        );
        // 200 + 50 is 250
        assert_matches!(
            fixture.create(priced_in(1, 50)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // 250 - 50 + 60
        let result = fixture.replace(priced_in(1, 60)).await;
        expect_violated(
            result,
            "categoryBudget",
            PropertyConstraintViolation::NotMet,
        );
        // Moved to category 2, the offer leaves 200 in category 1
        assert_matches!(
            fixture.replace(priced_in(2, 200)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture.create(priced_in(1, 50)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // Category 2 holds 200
        let result = fixture.create(priced_in(2, 51)).await;
        expect_violated(
            result,
            "categoryBudget",
            PropertyConstraintViolation::NotMet,
        );
        assert_eq!(fixture.stored_offers().len(), 4);
    }

    /// A transfer or a purchase counts the offer toward its new owner, so one
    /// reaching an owner at the cap is refused.
    #[tokio::test]
    async fn should_count_a_transferred_or_bought_offer_toward_its_new_owner() {
        let mut fixture = OfferFixture::with_schema(counted_offer_schema(platform_value!({
            "atMostOnePerOwner": {
                "lessThanOrEqual": [{ "countOf": ["offer", { "$ownerId": "$ownerId" }] }, 1]
            }
        })));
        let recipient = fixture.other_identity(971);
        assert_matches!(
            fixture.create(priced_in(1, 100)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture.transfer(recipient.0.id()).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // The writer owns none again, so it may list another
        assert_matches!(
            fixture.create(priced_in(1, 100)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let result = fixture.transfer(recipient.0.id()).await;
        expect_violated(
            result,
            "atMostOnePerOwner",
            PropertyConstraintViolation::NotMet,
        );

        fixture.set_price(1000).await;
        let result = fixture.purchase_by(&recipient, 1000).await;
        expect_violated(
            result,
            "atMostOnePerOwner",
            PropertyConstraintViolation::NotMet,
        );
        let buyer = fixture.other_identity(972);
        assert_matches!(
            fixture.purchase_by(&buyer, 1000).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let mut owners = fixture
            .stored_offers()
            .iter()
            .map(|offer| offer.owner_id())
            .collect::<Vec<_>>();
        owners.sort();
        let mut expected = vec![recipient.0.id(), buyer.0.id()];
        expected.sort();
        assert_eq!(owners, expected);
    }

    /// `countOf` over another type of the contract: an identity lists an offer
    /// only once it has a profile.
    #[tokio::test]
    async fn should_count_the_documents_of_another_type() {
        let mut fixture = OfferFixture::with_schemas(
            counted_offer_schema(platform_value!({
                "hasProfile": {
                    "greaterThanOrEqual": [
                        { "countOf": ["profile", { "$ownerId": "$ownerId" }] },
                        1
                    ]
                }
            })),
            [(
                "profile",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "handle": { "type": "string", "maxLength": 20, "position": 0 }
                    },
                    "required": ["handle"],
                    "indices": [{
                        "name": "byOwner",
                        "properties": [{ "$ownerId": "asc" }],
                        "countable": "countable"
                    }],
                    "additionalProperties": false
                }),
            )],
        );
        let result = fixture.create(priced_in(1, 100)).await;
        expect_violated(result, "hasProfile", PropertyConstraintViolation::NotMet);

        assert_matches!(
            fixture
                .create_of("profile", |document| {
                    document.set("handle", Value::Text("sam".to_string()))
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture.create(priced_in(1, 100)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// The totals a rule reads are state reads, billed with the write: the same
    /// create costs more when a rule reads a total than when it reads a
    /// property.
    #[tokio::test]
    async fn should_bill_the_totals_a_rule_reads() {
        let processing_fee = |result: StateTransitionExecutionResult| {
            let StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } = result
            else {
                panic!("expected the create to succeed, got {result:?}");
            };
            fee_result.processing_fee
        };
        let mut plain = OfferFixture::with_schema(counted_offer_schema(platform_value!({
            "rule": { "lessThanOrEqual": ["price", 1000] }
        })));
        let mut counted = OfferFixture::with_schema(counted_offer_schema(platform_value!({
            "rule": {
                "lessThanOrEqual": [{ "countOf": ["offer", { "$ownerId": "$ownerId" }] }, 1000]
            }
        })));
        let plain_fee = processing_fee(plain.create(priced_in(1, 100)).await);
        let counted_fee = processing_fee(counted.create(priced_in(1, 100)).await);
        assert!(
            counted_fee > plain_fee,
            "reading the count must be billed: {counted_fee} <= {plain_fee}"
        );
    }

    /// A price update is judged against the rules reading its update time; one
    /// that reads a total too reads it, so the time is judged, not skipped.
    #[tokio::test]
    async fn should_judge_a_price_update_by_a_rule_reading_a_total() {
        let mut schema = counted_offer_schema(platform_value!({
            "updatedBeforeEndAndFewOffers": {
                "allOf": [
                    { "lessThanOrEqual": ["$updatedAt", "endsAt"] },
                    {
                        "lessThanOrEqual": [
                            { "countOf": ["offer", { "$ownerId": "$ownerId" }] },
                            5
                        ]
                    }
                ]
            }
        }));
        let Value::Map(properties) = schema
            .get_mut("properties")
            .expect("properties")
            .expect("properties are set")
        else {
            panic!("properties is an object");
        };
        properties.push((
            Value::Text("endsAt".to_string()),
            platform_value!({ "type": "integer", "minimum": 0, "position": 5 }),
        ));
        let Value::Array(required) = schema
            .get_mut("required")
            .expect("required")
            .expect("required is set")
        else {
            panic!("required is an array");
        };
        required.push(Value::Text("endsAt".to_string()));
        required.push(Value::Text("$updatedAt".to_string()));
        let mut fixture = OfferFixture::with_schema(schema);
        fixture.block_info = at_block(NOW, 20);
        assert_matches!(
            fixture
                .create(|document| {
                    priced_in(1, 100)(document);
                    document.set("endsAt", Value::U64(NOW + DAY_MS));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        fixture.block_info = at_block(NOW + 2 * DAY_MS, 30);
        let result = fixture.try_set_price(1000).await;
        expect_violated(
            result,
            "updatedBeforeEndAndFewOffers",
            PropertyConstraintViolation::NotMet,
        );
        fixture.block_info = at_block(NOW + DAY_MS / 2, 30);
        fixture.set_price(1000).await;
    }

    /// `schema` with the document-type-level `entries` added.
    fn with_keys<const N: usize>(mut schema: Value, entries: [(&str, Value); N]) -> Value {
        let Value::Map(map) = &mut schema else {
            panic!("a schema is an object");
        };
        for (key, value) in entries {
            map.push((Value::Text(key.to_string()), value));
        }
        schema
    }

    /// `countOf` and `sumOf` over every document of the type, read from the
    /// primary-key trees `documentsCountable` and `documentsSummable` keep: at
    /// most two offers, whose prices total at most 250.
    #[tokio::test]
    async fn should_cap_the_whole_type_by_its_count_and_total() {
        let mut fixture = OfferFixture::with_schema(with_keys(
            counted_offer_schema(platform_value!({
                "fewOffers": { "lessThanOrEqual": [{ "countOf": ["offer"] }, 2] },
                "priceBudget": { "lessThanOrEqual": [{ "sumOf": ["offer", "price"] }, 250] }
            })),
            [
                ("documentsCountable", Value::Bool(true)),
                ("documentsSummable", Value::Text("price".to_string())),
            ],
        ));
        for category in [1, 2] {
            assert_matches!(
                fixture.create(priced_in(category, 100)).await,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }
        let result = fixture.create(priced_in(3, 10)).await;
        expect_violated(result, "fewOffers", PropertyConstraintViolation::NotMet);
        // 200 - 100 + 150, the count unchanged
        assert_matches!(
            fixture.replace(priced_in(2, 150)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // 250 - 150 + 151
        let result = fixture.replace(priced_in(2, 151)).await;
        expect_violated(result, "priceBudget", PropertyConstraintViolation::NotMet);
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// `countOf` by two keys, `$ownerId` and `category`: one offer per owner and
    /// category. The first read finds no branch for the owner yet, which reads
    /// as 0, and so does a category the owner has no offer in.
    #[tokio::test]
    async fn should_count_by_two_keys_from_an_empty_branch() {
        let mut fixture = OfferFixture::with_schema(counted_offer_schema(platform_value!({
            "onePerCategory": {
                "lessThanOrEqual": [
                    {
                        "countOf": [
                            "offer",
                            { "$ownerId": "$ownerId", "category": "category" }
                        ]
                    },
                    1
                ]
            }
        })));
        assert_matches!(
            fixture.create(priced_in(1, 100)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let result = fixture.create(priced_in(1, 100)).await;
        expect_violated(
            result,
            "onePerCategory",
            PropertyConstraintViolation::NotMet,
        );
        assert_matches!(
            fixture.create(priced_in(2, 100)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 2);
    }

    /// `sumOf` over another type of the contract: an offer's price is at most
    /// what the deposits of its category total.
    #[tokio::test]
    async fn should_total_a_property_of_another_type() {
        let mut fixture = OfferFixture::with_schemas(
            counted_offer_schema(platform_value!({
                "coveredByDeposits": {
                    "lessThanOrEqual": [
                        "price",
                        { "sumOf": ["deposit", "amount", { "category": "category" }] }
                    ]
                }
            })),
            [(
                "deposit",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100,
                            "position": 0
                        },
                        "amount": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 1000000000,
                            "position": 1
                        }
                    },
                    "required": ["category", "amount"],
                    "indices": [{
                        "name": "byCategory",
                        "properties": [{ "category": "asc" }],
                        "summable": "amount"
                    }],
                    "additionalProperties": false
                }),
            )],
        );
        let result = fixture.create(priced_in(1, 100)).await;
        expect_violated(
            result,
            "coveredByDeposits",
            PropertyConstraintViolation::NotMet,
        );
        assert_matches!(
            fixture
                .create_of("deposit", |document| {
                    document.set("category", Value::U64(1));
                    document.set("amount", Value::U64(150));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_matches!(
            fixture.create(priced_in(1, 100)).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let result = fixture.create(priced_in(2, 100)).await;
        expect_violated(
            result,
            "coveredByDeposits",
            PropertyConstraintViolation::NotMet,
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// The totals a rule reads leave out the other writes of its batch, which is
    /// sound only while a document batch carries one transition: raising the
    /// limit needs the batch's own writes added to them.
    #[test]
    fn should_keep_one_transition_per_document_batch_while_rules_read_totals() {
        assert_eq!(
            PlatformVersion::latest()
                .system_limits
                .max_transitions_in_documents_batch,
            1
        );
    }
}
