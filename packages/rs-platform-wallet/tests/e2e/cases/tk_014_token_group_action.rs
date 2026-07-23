//! TK-014 — Group-action gateway: queue a mint, list pending, co-sign.
//!
//! Three-identity contract whose `manualMintingRules` route through a
//! 2-of-3 group at position 0. Walks the gateway end-to-end:
//!   1. Identity #0 (owner) proposes a mint of `MINT_AMOUNT` to peer A.
//!   2. `pending_group_actions` lists the proposal (status
//!      `ActionActive`).
//!   3. Identity #1 (peer A) co-signs by re-broadcasting the same mint
//!      with `GroupStateTransitionInfoOtherSigner(action_id)`.
//!   4. After threshold, `pending_group_actions` shows the action
//!      `ActionClosed` and the recipient balance has moved.
//!
//! Wallet-feature parity: `wallet/identity/network/tokens/mint.rs:19`
//! (`token_mint_with_signer`) with `group_info: Some(...)`. The wallet
//! helper is a thin forward to `Sdk::token_mint`, so the test drives
//! the SDK builder directly — same chain path, mirrors the existing
//! `mint_to` framework helper.
//!
//! Group config is built inline (not via
//! `permissive_owner_token_contract_json`) because that builder
//! belongs to Wave 1; Wave 2 owns this case file's JSON shape.
//! `register_token_contract_via_sdk` doesn't surface a `groups`
//! injection point either, so this file ships its own
//! `publish_token_contract_with_groups` helper that mirrors the
//! framework helper's V1-envelope assembly with `groups` populated.
//!
//! Gated behind the `e2e` cargo feature for the same reason as transfer / TK-013.

use std::sync::Arc;
use std::time::Duration;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::{DataContract, GroupContractPosition};
use dpp::group::group_action_status::GroupActionStatus;
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

use dash_sdk::platform::tokens::builders::mint::TokenMintTransitionBuilder;
use dash_sdk::platform::tokens::transitions::MintResult;
use dash_sdk::platform::transition::put_contract::PutContract;
use dash_sdk::platform::Fetch;

use crate::framework::prelude::*;
use crate::framework::setup_with_per_identity_funding;
use crate::framework::tokens::{
    token_balance_of, token_supply_of, wait_for_token_balance, DEFAULT_BASE_SUPPLY,
    DEFAULT_DECIMALS, DEFAULT_MAX_SUPPLY, DEFAULT_TOKEN_POSITION, TK_OWNER_FUNDING_SIMPLE,
    TK_PEER_FUNDING_ACTIVE, TK_SETUP_WAIT_TIMEOUT,
};
use crate::framework::wallet_factory::RegisteredIdentity;

/// Tokens minted via the group-gated proposal. Small enough that any
/// arithmetic regression (extra credit, dropped co-sign) surfaces as
/// a stark balance mismatch.
const MINT_AMOUNT: TokenAmount = 42;

/// Group is at position 0 in the contract, threshold 2-of-3.
const GROUP_POSITION: GroupContractPosition = 0;

/// Replica-lag budget for the post-cosign balance gate. Same 60 s the
/// sibling token tests (TK-001/004/007/008/009/011) use.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_014_token_group_action_mint_co_sign() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    {
        let floor_ctx = E2eContext::init().await.expect("init e2e context");
        if floor_ctx.skip_if_bank_floor_unmet("tk_014") {
            return;
        }
    }

    // Register three identities only — TK-014 needs a group-gated
    // contract that the framework's `setup_with_token_and_three_identities`
    // helper does not yet support, so we skip the helper's
    // permissive-contract deploy and publish the group-gated contract
    // ourselves below. Saves one full contract-create fee per run.
    //
    // QA-V40-001 — three concurrent 35 B-credit funding waits under
    // `worker_threads = 12` shared-bank churn exceed the 60 s
    // `DEFAULT_SETUP_STEP_TIMEOUT`. Route through the explicit-budget
    // entry point with `TK_SETUP_WAIT_TIMEOUT` (120 s), mirroring the
    // five `setup_with_token_*` helpers and TK-003 / TK-013.
    let setup_guard = setup_with_per_identity_funding(
        &[
            TK_OWNER_FUNDING_SIMPLE,
            TK_PEER_FUNDING_ACTIVE,
            TK_PEER_FUNDING_ACTIVE,
        ],
        TK_SETUP_WAIT_TIMEOUT,
    )
    .await
    .expect("register three identities");
    let ctx = setup_guard.base.ctx;
    let owner = &setup_guard.identities[0];
    let peer_a = &setup_guard.identities[1];
    let peer_b = &setup_guard.identities[2];
    let recipient_id = peer_a.id;

    let group_member_ids = [owner.id, peer_a.id, peer_b.id];
    let contract_id = publish_token_contract_with_groups(ctx, owner, &group_member_ids)
        .await
        .expect("publish group-gated token contract");

    // Snapshot baseline. Token max supply is the harness default,
    // base supply zero — both balance and supply start at 0; the
    // assertions still diff against the snapshot to stay robust
    // against any historical seed.
    let supply_before = token_supply_of(ctx, contract_id, DEFAULT_TOKEN_POSITION)
        .await
        .expect("pre-propose total supply");
    let balance_before = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, recipient_id)
        .await
        .expect("pre-propose recipient balance");

    let data_contract: Arc<DataContract> = Arc::new(
        DataContract::fetch(ctx.sdk(), contract_id)
            .await
            .expect("fetch token data contract")
            .expect("token data contract present on chain"),
    );

    // Step 1 — owner proposes a mint via the group gateway.
    let propose_result = mint_with_group_info(
        ctx,
        Arc::clone(&data_contract),
        owner,
        recipient_id,
        MINT_AMOUNT,
        GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(GROUP_POSITION),
    )
    .await
    .expect("owner propose mint");

    // After step 1 the recipient balance must be unchanged — the
    // proposal sits below the threshold and tokens haven't moved.
    let balance_mid = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, recipient_id)
        .await
        .expect("post-propose recipient balance");
    assert_eq!(
        balance_mid, balance_before,
        "recipient balance must not change before threshold is met (observed before={balance_before} mid={balance_mid})"
    );

    // Step 2 — list pending group actions; assert one entry with the
    // proposed amount + recipient.
    let pending = pending_group_actions(
        ctx.sdk(),
        contract_id,
        GROUP_POSITION,
        GroupActionStatus::ActionActive,
    )
    .await
    .expect("list pending group actions");

    let active_entry = pending
        .iter()
        .find(|e| {
            matches!(&e.params, GroupActionParamsLite::Mint { amount, recipient }
            if *amount == MINT_AMOUNT && *recipient == recipient_id)
        })
        .expect("pending list must contain the proposed mint");

    let action_id = active_entry.action_id;
    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_014",
        ?action_id,
        proposer = ?active_entry.proposer,
        "TK-014 proposed action surfaced in pending list"
    );

    // Cross-reference the result of step 1: the proposer leg must
    // produce a group-action shape (never the synchronous
    // TokenBalance/HistoricalDocument shape — those would mean the
    // proposal already executed, contradicting the 2-of-3 threshold).
    match &propose_result {
        MintResult::GroupActionWithBalance(_, status, _) => {
            assert_eq!(
                *status,
                GroupActionStatus::ActionActive,
                "proposer leg must leave the action ActionActive (observed {status:?})"
            );
        }
        MintResult::GroupActionWithDocument(_, _) => {}
        MintResult::TokenBalance(_, _) | MintResult::HistoricalDocument(_) => {
            panic!("proposer leg must NOT produce a synchronous mint result — that would bypass the 2-of-3 group threshold");
        }
    }

    // Step 3 — peer A co-signs. Same builder, group_info points at
    // the existing action_id with `action_is_proposer = false`.
    let co_sign_result = mint_with_group_info(
        ctx,
        Arc::clone(&data_contract),
        peer_a,
        recipient_id,
        MINT_AMOUNT,
        GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
            GroupStateTransitionInfo {
                group_contract_position: GROUP_POSITION,
                action_id,
                action_is_proposer: false,
            },
        ),
    )
    .await
    .expect("peer A co-sign mint");

    // Mirror the sibling token tests (TK-001/004/007/008/009/011):
    // gate the post-cosign reads on the recipient's token balance
    // reaching the expected post-mint amount. The co-sign ST closes
    // the group action and credits the recipient; reading immediately
    // can hit a DAPI replica that hasn't yet applied the block and
    // return the pre-mint value, racing the balance + supply
    // assertions below.
    wait_for_token_balance(
        ctx,
        recipient_id,
        contract_id,
        DEFAULT_TOKEN_POSITION,
        balance_before + MINT_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("recipient balance never reached pre-mint + minted amount");

    // Step 4 — recipient balance and supply must have advanced now
    // that the threshold (2-of-3) is met.
    let balance_after = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, recipient_id)
        .await
        .expect("post-cosign recipient balance");
    let supply_after = token_supply_of(ctx, contract_id, DEFAULT_TOKEN_POSITION)
        .await
        .expect("post-cosign total supply");

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_014",
        ?contract_id,
        ?recipient_id,
        ?action_id,
        balance_before,
        balance_mid,
        balance_after,
        supply_before,
        supply_after,
        amount = MINT_AMOUNT,
        "TK-014 post-cosign balance + supply snapshot"
    );

    assert_eq!(
        balance_after,
        balance_before + MINT_AMOUNT,
        "recipient balance must advance by the minted amount after threshold is met. \
         observed before={balance_before} after={balance_after} expected_delta={MINT_AMOUNT}"
    );
    assert_eq!(
        supply_after,
        supply_before + MINT_AMOUNT,
        "total supply must advance by the minted amount after threshold is met. \
         observed before={supply_before} after={supply_after} expected_delta={MINT_AMOUNT}"
    );

    // Pending list now reports the action as Closed.
    let closed = pending_group_actions(
        ctx.sdk(),
        contract_id,
        GROUP_POSITION,
        GroupActionStatus::ActionClosed,
    )
    .await
    .expect("list closed group actions");
    assert!(
        closed.iter().any(|e| e.action_id == action_id),
        "closed-action list must contain the just-completed action_id={action_id}"
    );

    // Active list must no longer carry the action_id.
    let still_active = pending_group_actions(
        ctx.sdk(),
        contract_id,
        GROUP_POSITION,
        GroupActionStatus::ActionActive,
    )
    .await
    .expect("re-list active group actions");
    assert!(
        still_active.iter().all(|e| e.action_id != action_id),
        "active-action list must NOT carry the closed action_id={action_id}"
    );

    // Sanity-check the co-sign result envelope.
    match &co_sign_result {
        MintResult::GroupActionWithBalance(_, status, _) => {
            assert_eq!(
                *status,
                GroupActionStatus::ActionClosed,
                "co-sign leg that meets threshold must close the action (observed {status:?})"
            );
        }
        // History-tracked tokens take the document arm; the closed
        // status is implicit in the balance/supply assertions above.
        MintResult::GroupActionWithDocument(_, _) => {}
        MintResult::TokenBalance(_, _) | MintResult::HistoricalDocument(_) => {
            panic!("co-sign leg must produce a group-action MintResult");
        }
    }

    setup_guard.teardown().await.expect("teardown");
}

/// Drive `Sdk::token_mint` with the supplied `group_info`. Mirrors
/// the wallet's `token_mint_with_signer` (`mint.rs:19`) — that helper
/// just forwards to the SDK with the same `with_using_group_info`
/// hook, which is what we drive here to keep the test surface flat.
async fn mint_with_group_info(
    ctx: &E2eContext,
    data_contract: Arc<DataContract>,
    actor: &RegisteredIdentity,
    recipient_id: Identifier,
    amount: TokenAmount,
    group_info: GroupStateTransitionInfoStatus,
) -> Result<MintResult, dash_sdk::Error> {
    let builder =
        TokenMintTransitionBuilder::new(data_contract, DEFAULT_TOKEN_POSITION, actor.id, amount)
            .issued_to_identity_id(recipient_id)
            .with_using_group_info(group_info);
    ctx.sdk()
        .token_mint(builder, &actor.critical_key, actor.signer.as_ref())
        .await
}

/// Flattened mint-only view over a pending group action. Drops the
/// fields TK-014 doesn't read (public_note, position) so the
/// assertion site stays compact.
struct PendingActionLite {
    action_id: Identifier,
    proposer: Identifier,
    params: GroupActionParamsLite,
}

enum GroupActionParamsLite {
    Mint {
        amount: TokenAmount,
        recipient: Identifier,
    },
    Other,
}

/// Local wrapper around `GroupAction::fetch_many` filtered to
/// `(contract, position, status)`. The wallet's
/// `pending_group_actions_external` helper does the same thing in
/// production code; we mirror it inline so the test crate stays free
/// of platform-wallet's internal-helper dependency.
async fn pending_group_actions(
    sdk: &dash_sdk::Sdk,
    contract_id: Identifier,
    group_contract_position: GroupContractPosition,
    status: GroupActionStatus,
) -> Result<Vec<PendingActionLite>, dash_sdk::Error> {
    use dash_sdk::platform::group_actions::GroupActionsQuery;
    use dash_sdk::platform::FetchMany;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::{GroupAction, GroupActionAccessors};
    use dpp::tokens::token_event::TokenEvent;

    let query = GroupActionsQuery {
        contract_id,
        group_contract_position,
        status,
        start_at_action_id: None,
        limit: None,
    };

    let rows = GroupAction::fetch_many(sdk, query).await?;
    let mut out = Vec::with_capacity(rows.len());
    for (action_id, maybe_action) in rows {
        let Some(action) = maybe_action else { continue };
        let GroupActionEvent::TokenEvent(event) = action.event().clone();
        let params = match event {
            TokenEvent::Mint(amount, recipient, _note) => {
                GroupActionParamsLite::Mint { amount, recipient }
            }
            _ => GroupActionParamsLite::Other,
        };
        out.push(PendingActionLite {
            action_id,
            proposer: action.proposer_id(),
            params,
        });
    }
    Ok(out)
}

/// Inline V1-envelope assembler. Mirrors the framework's
/// `register_token_contract_via_sdk` (Wave 1) but injects the
/// `groups` field — which the framework helper currently doesn't
/// surface. Returns the chain-confirmed contract id.
async fn publish_token_contract_with_groups(
    ctx: &E2eContext,
    owner: &RegisteredIdentity,
    group_members: &[Identifier; 3],
) -> FrameworkResult<Identifier> {
    use serde_json::json;

    let placeholder_id = Identifier::default();
    let owner_b58 = bs58::encode(owner.id.to_buffer()).into_string();

    let owner_only = json!({
        "$formatVersion": "0",
        "authorizedToMakeChange": {"$type": "contractOwner"},
        "adminActionTakers": {"$type": "contractOwner"},
        "changingAuthorizedActionTakersToNoOneAllowed": false,
        "changingAdminActionTakersToNoOneAllowed": false,
        "selfChangingAdminActionTakersAllowed": false,
    });
    let group_only = json!({
        "$formatVersion": "0",
        "authorizedToMakeChange": {"$type": "group", "position": GROUP_POSITION},
        "adminActionTakers": {"$type": "group", "position": GROUP_POSITION},
        "changingAuthorizedActionTakersToNoOneAllowed": false,
        "changingAdminActionTakersToNoOneAllowed": false,
        "selfChangingAdminActionTakersAllowed": false,
    });

    // `serde_json::json!` requires literal map keys, so build the
    // member roster manually.
    let mut members = serde_json::Map::new();
    for id in group_members {
        members.insert(bs58::encode(id.to_buffer()).into_string(), json!(1u32));
    }
    let group = json!({
        "$formatVersion": "0",
        "members": serde_json::Value::Object(members),
        "requiredPower": 2u32,
    });
    let mut groups = serde_json::Map::new();
    groups.insert(GROUP_POSITION.to_string(), group);

    let token_slot = json!({
        "$formatVersion": "0",
        "conventions": {
            "$formatVersion": "0",
            "decimals": DEFAULT_DECIMALS,
            "localizations": {
                "en": {
                    "$formatVersion": "0",
                    "shouldCapitalize": false,
                    "singularForm": "E2ETestToken",
                    "pluralForm": "E2ETestTokens",
                }
            },
        },
        "conventionsChangeRules": owner_only,
        "baseSupply": DEFAULT_BASE_SUPPLY,
        "maxSupply": DEFAULT_MAX_SUPPLY,
        "keepsHistory": {
            "$formatVersion": "0",
            "keepsTransferHistory": true,
            "keepsFreezingHistory": true,
            "keepsMintingHistory": true,
            "keepsBurningHistory": true,
            "keepsDirectPricingHistory": true,
            "keepsDirectPurchaseHistory": true,
        },
        "startAsPaused": false,
        "allowTransferToFrozenBalance": false,
        "maxSupplyChangeRules": owner_only,
        "distributionRules": {
            "$formatVersion": "0",
            "perpetualDistribution": null,
            "perpetualDistributionRules": owner_only,
            "preProgrammedDistribution": null,
            "newTokensDestinationIdentity": owner_b58,
            "newTokensDestinationIdentityRules": owner_only,
            "mintingAllowChoosingDestination": true,
            "mintingAllowChoosingDestinationRules": owner_only,
            "changeDirectPurchasePricingRules": owner_only,
        },
        // The whole point of TK-014: gate manual minting on the
        // 2-of-3 group at position 0.
        "manualMintingRules": group_only,
        "manualBurningRules": owner_only,
        "freezeRules": owner_only,
        "unfreezeRules": owner_only,
        "destroyFrozenFundsRules": owner_only,
        "emergencyActionRules": owner_only,
        "mainControlGroup": GROUP_POSITION,
        "mainControlGroupCanBeModified": {"$type": "contractOwner"},
        "description": "TK-014 group-gated mint token (rs-platform-wallet e2e).",
        "marketplaceRules": {
            "$formatVersion": "0",
            "tradeMode": "NotTradeable",
            "tradeModeChangeRules": owner_only,
        },
    });

    let mut tokens = serde_json::Map::new();
    tokens.insert(DEFAULT_TOKEN_POSITION.to_string(), token_slot);

    let mut envelope = serde_json::Map::new();
    envelope.insert("$formatVersion".into(), json!("1"));
    envelope.insert(
        "id".into(),
        json!(bs58::encode(placeholder_id.to_buffer()).into_string()),
    );
    envelope.insert("ownerId".into(), json!(owner_b58));
    envelope.insert("version".into(), json!(1u32));
    envelope.insert("documentSchemas".into(), json!({}));
    envelope.insert("groups".into(), serde_json::Value::Object(groups));
    envelope.insert("tokens".into(), serde_json::Value::Object(tokens));

    let serialized = serde_json::to_string(&serde_json::Value::Object(envelope))
        .map_err(|err| FrameworkError::Sdk(format!("token-contract serialize: {err}")))?;
    let format: DataContractInSerializationFormat = serde_json::from_str(&serialized)
        .map_err(|err| FrameworkError::Sdk(format!("token-contract deserialize: {err}")))?;

    let platform_version = PlatformVersion::latest();
    let mut errors = vec![];
    let data_contract =
        DataContract::try_from_platform_versioned(format, true, &mut errors, platform_version)
            .map_err(|err| {
                FrameworkError::Sdk(format!("token-contract build: {err} (errors={errors:?})"))
            })?;

    let confirmed = data_contract
        .put_to_platform_and_wait_for_response(
            ctx.sdk(),
            owner.high_key.clone(),
            owner.signer.as_ref(),
            None,
        )
        .await
        .map_err(|err| FrameworkError::Sdk(format!("put_to_platform: {err}")))?;

    let contract_id = confirmed.id();

    crate::framework::wait::wait_for_data_contract_visible(
        ctx.sdk(),
        contract_id,
        std::time::Duration::from_secs(60),
        2,
    )
    .await?;

    // QA-900 — same register-with-trusted-context dance as
    // `register_token_contract_via_sdk`. TK-014 publishes its
    // group-gated contract inline (the framework helper doesn't
    // surface a `groups` injection point), so the registration has
    // to happen here too — otherwise `mint_with_group_info` lands on
    // `DriveProofError(UnknownContract)`.
    crate::framework::tokens::register_contract_with_context_provider(ctx, &confirmed);

    Ok(contract_id)
}
