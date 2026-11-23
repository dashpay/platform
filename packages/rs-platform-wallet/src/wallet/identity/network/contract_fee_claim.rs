//! Claim a data contract's fee pots on `IdentityWallet` (protocol version 14).
//!
//! A document type may charge a fee for actions on its documents (`actionFees`); the owner
//! part collects in the contract's owner pot and the moderators part in its moderators pot. A
//! `ContractFeeClaim` transition pays a pot out. rs-sdk builds, signs and proves the claim
//! (`dash_sdk::platform::transition::contract_fee_claim`); Drive decides whether it is
//! allowed, whether the pot was already paid out this epoch and how it is split. This module
//! orchestrates the claim for an identity this wallet holds and keeps the balances the proof
//! shows, and answers which of the wallet's identities may claim which pot, from the owners
//! of that knowledge: dpp's [`ContractFeePot::recipients`] for the team a contract names, and
//! rs-sdk's [`ModerationTeam`] for an elected contract's seated team.

use std::collections::BTreeSet;

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::moderation_charters::ModerationTeam;
use dash_sdk::platform::transition::contract_fee_claim::{
    ClaimContractFeesWithMetadata, ClaimedContractFees,
};
use dash_sdk::platform::Fetch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::{DataContract, Identifier};

use crate::error::PlatformWalletError;

use super::document::SignerRef;
use super::*;

/// Who a contract's moderators pot belongs to when it is read, which decides who may claim
/// it. Drive makes the same choice when it validates a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeratorsPotTeam {
    /// The contract declares no moderation, so it has no team and nobody claims its
    /// moderators pot.
    NoModeration,
    /// The team the contract's moderation declaration names: its owner or its appointed
    /// moderators ([`ContractFeePot::recipients`]).
    Declared,
    /// An elected contract with no seated charter: the interim team its declaration names
    /// ([`ContractFeePot::recipients`]), which may be nobody.
    ElectedInterim,
    /// An elected contract with a seated charter: the charter's leader and its active
    /// members ([`ModerationTeam::contains`]), read from the moderation charters contract.
    ElectedSeated,
}

/// Which identities of a wallet may claim which fee pot of a contract, as the contract and,
/// for an elected contract, the moderation charters contract read when this was asked.
///
/// It says who may sign a claim, nothing more: whether the pot holds enough to pay everyone
/// something and whether it was already paid out in the current epoch are Drive's checks at
/// claim time, refused with their consensus errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractFeeClaimEligibility {
    /// The contract asked about.
    pub contract_id: Identifier,
    /// The wallet's identities that may claim the owner pot: the contract owner, when this
    /// wallet holds it.
    pub owner_pot_claimants: BTreeSet<Identifier>,
    /// The wallet's identities that may claim the moderators pot, for the whole team.
    pub moderators_pot_claimants: BTreeSet<Identifier>,
    /// Who the moderators pot belongs to.
    pub moderators_pot_team: ModeratorsPotTeam,
}

impl IdentityWallet {
    /// Pay out `pot` of `contract_id`, signed by `claimant_identity_id`, an identity this
    /// wallet holds, and return the claim as its proof shows it: the pot's last claim, the
    /// credits left in it and the balances it paid.
    ///
    /// rs-sdk picks the signing key (the claimant's first CRITICAL authentication key without
    /// contract bounds that `signer` can sign with), fetches the contract again for the proof
    /// check, signs, broadcasts and waits for the proved pot and balances. The balance of
    /// every identity of this wallet the proof shows, the claimant's included, is kept as the
    /// proof's block confirms it. The signer is called with the wallet manager lock released.
    ///
    /// # Errors
    /// * [`PlatformWalletError::IdentityNotFound`] when this wallet holds no signing identity
    ///   `claimant_identity_id`;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures. Platform's refusals keep
    ///   their consensus error: a claimant that is not a recipient of the pot
    ///   (`ContractFeeClaimNotAllowedError`), a pot already paid out in this epoch
    ///   (`ContractFeesAlreadyClaimedThisEpochError`), a pot too small to pay everyone a
    ///   credit (`ContractFeesNothingToClaimError`) and a contract that does not exist.
    pub async fn claim_contract_fees_with_signer<S>(
        &self,
        claimant_identity_id: &Identifier,
        contract_id: &Identifier,
        pot: ContractFeePot,
        signer: &S,
    ) -> Result<ClaimedContractFees, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let claimant = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager
                .wallet_identity(&self.wallet_id, claimant_identity_id)
                .map(|managed| managed.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*claimant_identity_id))?
        };

        let (claimed, metadata) = claimant
            .claim_contract_fees_with_metadata(
                &self.sdk,
                *contract_id,
                pot,
                None,
                SignerRef(signer),
                None,
            )
            .await?;

        self.persist_claimed_balances(&claimed, &metadata).await;
        Ok(claimed)
    }

    /// Keep the balance of every identity of this wallet that the proof of `claimed` shows,
    /// as of the proof's block (`metadata`); the balances of identities this wallet does not
    /// hold are ignored.
    async fn persist_claimed_balances(
        &self,
        claimed: &ClaimedContractFees,
        metadata: &ResponseMetadata,
    ) {
        for (identity_id, balance) in &claimed.balances {
            self.persist_proven_owner_balance(identity_id, Some(*balance), metadata.clone())
                .await;
        }
    }

    /// Which of this wallet's signing identities may claim which fee pot of `contract_id`.
    ///
    /// Reads the contract and, when it declares elected moderation, the team of its seated
    /// charter (`Sdk::fetch_moderation_team`), then answers with
    /// [`contract_fee_claim_eligibility`]. A host offers a claim only to the identities named
    /// here; Drive still checks the claim when it is sent.
    ///
    /// # Errors
    /// * [`PlatformWalletError::ContractNotFound`] when Platform holds no such contract;
    /// * [`PlatformWalletError::Sdk`] when a read fails.
    pub async fn contract_fee_claim_eligibility(
        &self,
        contract_id: &Identifier,
    ) -> Result<ContractFeeClaimEligibility, PlatformWalletError> {
        let contract = DataContract::fetch(&self.sdk, *contract_id).await?.ok_or(
            PlatformWalletError::ContractNotFound {
                contract_id: *contract_id,
            },
        )?;
        let seated_team = if declares_elected_moderation(&contract) {
            self.sdk.fetch_moderation_team(*contract_id).await?
        } else {
            None
        };
        let wallet_identities = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager.wallet_identity_ids(&self.wallet_id)
        };
        Ok(contract_fee_claim_eligibility(
            &contract,
            seated_team.as_ref(),
            wallet_identities,
        ))
    }
}

/// Whether `contract` declares elected moderation, the case in which Drive reads the seated
/// charter before it judges a claim of the moderators pot.
fn declares_elected_moderation(contract: &DataContract) -> bool {
    contract
        .config()
        .moderation()
        .and_then(|moderation| moderation.moderators.elected())
        .is_some()
}

/// Which of `candidates` may claim which fee pot of `contract`, given the team of the
/// contract's seated charter (`seated_team`, read only for a contract that declares elected
/// moderation; ignored for any other).
///
/// The owner pot is claimed by its recipient, the contract owner. The moderators pot of an
/// elected contract with a seated charter is claimed by the charter's leader or an active
/// member ([`ModerationTeam::contains`]); otherwise by the recipients the contract names
/// ([`ContractFeePot::recipients`]), which is nobody for a contract that declares no
/// moderation. This is the choice Drive's claim validation makes.
pub fn contract_fee_claim_eligibility(
    contract: &DataContract,
    seated_team: Option<&ModerationTeam>,
    candidates: impl IntoIterator<Item = Identifier>,
) -> ContractFeeClaimEligibility {
    let candidates: BTreeSet<Identifier> = candidates.into_iter().collect();
    let owner_recipients = ContractFeePot::Owner.recipients(contract);
    let owner_pot_claimants = candidates
        .intersection(&owner_recipients)
        .copied()
        .collect();

    let elected = declares_elected_moderation(contract);
    let (moderators_pot_team, moderators_pot_claimants) = match (elected, seated_team) {
        (true, Some(team)) => (
            ModeratorsPotTeam::ElectedSeated,
            candidates
                .iter()
                .filter(|candidate| team.contains(candidate))
                .copied()
                .collect(),
        ),
        _ => {
            let team = if elected {
                ModeratorsPotTeam::ElectedInterim
            } else if contract.config().moderation().is_some() {
                ModeratorsPotTeam::Declared
            } else {
                ModeratorsPotTeam::NoModeration
            };
            let recipients = ContractFeePot::Moderators.recipients(contract);
            (
                team,
                candidates.intersection(&recipients).copied().collect(),
            )
        }
    };

    ContractFeeClaimEligibility {
        contract_id: contract.id(),
        owner_pot_claimants,
        moderators_pot_claimants,
        moderators_pot_team,
    }
}

#[cfg(test)]
mod tests {
    use super::super::decision_proposal::tests::{owner, proposals_sdk, FixedSignatureSigner};
    use super::super::signing_key::tests::{lock_checking_signer, wallet_with_signing_keys_on};
    use super::*;
    use crate::wallet::persister::NoPlatformPersistence;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
    use dash_sdk::query_types::IdentityContractNonceFetcher;
    use dpp::consensus::state::contract_moderation::ContractFeesAlreadyClaimedThisEpochError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::moderation::elected::{ElectedModerators, InterimModerators};
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::action_fees::ContractFeePotLastClaim;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::state_transition::contract_fee_claim_transition::methods::ContractFeeClaimTransitionMethodsV0;
    use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const STORED_NONCE: u64 = 9;

    fn id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    const OWNER: u8 = 0x0A;

    /// A contract of `OWNER` declaring `moderators`, or no moderation at all.
    fn contract_with(moderators: Option<ContractModerators>) -> DataContract {
        let mut contract = get_data_contract_fixture(
            Some(id(OWNER)),
            0,
            PlatformVersion::latest().protocol_version,
        )
        .data_contract_owned();
        contract.set_owner_id(id(OWNER));
        let moderation = moderators.map(|moderators| ContractModerationConfig {
            banlist: true,
            suspensions: false,
            moderators,
            warnings: false,
        });
        let config: DataContractConfig = contract.config().clone();
        contract.set_config(config.with_moderation(moderation));
        contract
    }

    fn elected(interim: InterimModerators) -> ContractModerators {
        ContractModerators::Elected(Box::new(ElectedModerators {
            join_window: 0,
            vote_window: 0,
            challenge_cool_down: None,
            election_delay: None,
            max_added_moderators: 2,
            moderated_document_types: BTreeMap::new(),
            interim,
            owner_protected: false,
        }))
    }

    /// A seated team led by `leader` with the active `members`.
    fn seated(leader: u8, members: &[u8]) -> ModerationTeam {
        ModerationTeam {
            elected_charter_id: id(0xC1),
            submitted_charter_id: id(0xC2),
            leader_id: id(leader),
            members: members.iter().copied().map(id).collect(),
            elected_members: members.iter().copied().map(id).collect(),
        }
    }

    fn ids(bytes: &[u8]) -> BTreeSet<Identifier> {
        bytes.iter().copied().map(id).collect()
    }

    #[test]
    fn should_let_only_the_owner_claim_the_owner_pot() {
        let contract = contract_with(None);
        let held = contract_fee_claim_eligibility(&contract, None, [id(OWNER), id(2)]);
        assert_eq!(held.owner_pot_claimants, ids(&[OWNER]));
        assert_eq!(held.contract_id, contract.id());

        let not_held = contract_fee_claim_eligibility(&contract, None, [id(2), id(3)]);
        assert!(not_held.owner_pot_claimants.is_empty());
    }

    #[test]
    fn should_let_nobody_claim_the_moderators_pot_of_a_contract_without_moderation() {
        let contract = contract_with(None);
        let eligibility = contract_fee_claim_eligibility(&contract, None, [id(OWNER), id(2)]);
        assert_eq!(
            eligibility.moderators_pot_team,
            ModeratorsPotTeam::NoModeration
        );
        assert!(eligibility.moderators_pot_claimants.is_empty());
    }

    /// The candidates the contract names, as dpp's `recipients` reads the declaration: the
    /// owner for an owner-moderated contract, exactly the appointed set otherwise.
    #[test]
    fn should_let_the_declared_team_claim_the_moderators_pot() {
        let owner_moderated = contract_with(Some(ContractModerators::ContractOwner));
        let eligibility =
            contract_fee_claim_eligibility(&owner_moderated, None, [id(OWNER), id(2)]);
        assert_eq!(eligibility.moderators_pot_team, ModeratorsPotTeam::Declared);
        assert_eq!(eligibility.moderators_pot_claimants, ids(&[OWNER]));

        let appointed = contract_with(Some(ContractModerators::AppointedModerators(ids(&[2, 3]))));
        let eligibility =
            contract_fee_claim_eligibility(&appointed, None, [id(OWNER), id(2), id(4)]);
        assert_eq!(eligibility.moderators_pot_team, ModeratorsPotTeam::Declared);
        assert_eq!(
            eligibility.moderators_pot_claimants,
            ContractFeePot::Moderators
                .recipients(&appointed)
                .intersection(&ids(&[OWNER, 2, 4]))
                .copied()
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(eligibility.moderators_pot_claimants, ids(&[2]));
    }

    /// A seated team passed for a contract that is not elected is not read: Drive reads a
    /// charter only for an elected contract.
    #[test]
    fn should_ignore_a_seated_team_for_a_contract_that_is_not_elected() {
        let contract = contract_with(Some(ContractModerators::ContractOwner));
        let team = seated(5, &[6]);
        let eligibility =
            contract_fee_claim_eligibility(&contract, Some(&team), [id(OWNER), id(5), id(6)]);
        assert_eq!(eligibility.moderators_pot_team, ModeratorsPotTeam::Declared);
        assert_eq!(eligibility.moderators_pot_claimants, ids(&[OWNER]));
    }

    #[test]
    fn should_let_the_interim_team_claim_until_a_charter_is_seated() {
        let contract = contract_with(Some(elected(InterimModerators::AppointedModerators(ids(
            &[2],
        )))));
        let eligibility = contract_fee_claim_eligibility(&contract, None, [id(OWNER), id(2)]);
        assert_eq!(
            eligibility.moderators_pot_team,
            ModeratorsPotTeam::ElectedInterim
        );
        assert_eq!(eligibility.moderators_pot_claimants, ids(&[2]));

        let unmoderated_interim = contract_with(Some(elected(InterimModerators::NoModeration)));
        let eligibility =
            contract_fee_claim_eligibility(&unmoderated_interim, None, [id(OWNER), id(2)]);
        assert_eq!(
            eligibility.moderators_pot_team,
            ModeratorsPotTeam::ElectedInterim
        );
        assert!(eligibility.moderators_pot_claimants.is_empty());
    }

    /// Once a charter is seated its leader and active members claim, and the interim team,
    /// the owner among it, no longer does, as Drive's seated claim checks.
    #[test]
    fn should_let_the_seated_leader_and_active_members_claim_and_not_the_interim_team() {
        let contract = contract_with(Some(elected(InterimModerators::ContractOwner)));
        let team = seated(5, &[6, 7]);
        let eligibility = contract_fee_claim_eligibility(
            &contract,
            Some(&team),
            [id(OWNER), id(5), id(6), id(8)],
        );
        assert_eq!(
            eligibility.moderators_pot_team,
            ModeratorsPotTeam::ElectedSeated
        );
        assert_eq!(eligibility.moderators_pot_claimants, ids(&[5, 6]));
        // The owner pot stays the owner's whoever moderates
        assert_eq!(eligibility.owner_pot_claimants, ids(&[OWNER]));
    }

    /// A contract Platform does not hold is its own error, before anything else is read.
    #[tokio::test]
    async fn should_report_a_contract_platform_does_not_hold() {
        let contract_id = id(0xEE);
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract_id, None::<DataContract>)
            .await
            .expect("expects the contract read");
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .contract_fee_claim_eligibility(&contract_id)
            .await
            .expect_err("no such contract");
        assert!(
            matches!(error, PlatformWalletError::ContractNotFound { contract_id: missing }
                if missing == contract_id),
            "{error:?}"
        );
    }

    /// The wallet's signing identity (the `owner()` the fixture holds) is offered the owner
    /// pot of a contract it owns, read once through the SDK; a contract that is not elected
    /// reads no charter (the mock answers nothing else).
    #[tokio::test]
    async fn should_offer_the_owner_pot_to_the_wallet_identity_that_owns_the_contract() {
        let mut contract = contract_with(Some(ContractModerators::ContractOwner));
        contract.set_owner_id(owner());
        let contract_id = contract.id();
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract_id, Some(contract))
            .await
            .expect("expects the contract read");
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;

        let eligibility = wallet
            .identity()
            .contract_fee_claim_eligibility(&contract_id)
            .await
            .expect("reads");
        assert_eq!(eligibility.contract_id, contract_id);
        assert_eq!(eligibility.owner_pot_claimants, BTreeSet::from([owner()]));
        assert_eq!(
            eligibility.moderators_pot_claimants,
            BTreeSet::from([owner()])
        );
        assert_eq!(eligibility.moderators_pot_team, ModeratorsPotTeam::Declared);
    }

    /// A claimant the wallet does not hold is refused before anything is read or signed.
    #[tokio::test]
    async fn should_refuse_a_claimant_the_wallet_does_not_hold() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        let stranger = id(5);
        let error = wallet
            .identity()
            .claim_contract_fees_with_signer(
                &stranger,
                &id(0xEE),
                ContractFeePot::Owner,
                &FixedSignatureSigner,
            )
            .await
            .expect_err("not an identity of this wallet");
        assert!(
            matches!(error, PlatformWalletError::IdentityNotFound(missing) if missing == stranger),
            "{error:?}"
        );
    }

    /// rs-sdk asks the signer for a key with the wallet manager lock released; a signer
    /// holding no CRITICAL key of the claimant stops the claim before any request (the mock
    /// SDK answers none).
    #[tokio::test]
    async fn should_ask_the_signer_for_a_key_with_the_wallet_lock_released() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        let signer = lock_checking_signer(wallet.identity());
        let error = wallet
            .identity()
            .claim_contract_fees_with_signer(
                &owner(),
                &id(0xEE),
                ContractFeePot::Moderators,
                &signer,
            )
            .await
            .expect_err("no key available");
        assert!(
            matches!(error, PlatformWalletError::Sdk(dash_sdk::Error::Generic(ref message))
                if message.contains("CRITICAL authentication key")),
            "{error:?}"
        );
    }

    /// The claim reads the contract again and the claimant's next contract nonce, then
    /// broadcasts the claim of the pot signed with the claimant's first CRITICAL key. The
    /// mock answers no broadcast, so the claim fails after sending it.
    #[tokio::test]
    async fn should_broadcast_the_claim_of_the_pot_signed_by_the_claimant() {
        let mut contract = contract_with(Some(ContractModerators::ContractOwner));
        contract.set_owner_id(owner());
        let contract_id = contract.id();
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract_id, Some(contract))
            .await
            .expect("expects the contract read");
        sdk.mock()
            .expect_fetch(
                (owner(), contract_id),
                Some(IdentityContractNonceFetcher(STORED_NONCE)),
            )
            .await
            .expect("expects the nonce read");
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .claim_contract_fees_with_signer(
                &owner(),
                &contract_id,
                ContractFeePot::Owner,
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");

        let claimant = {
            let wm = wallet.identity().wallet_manager.read().await;
            wm.get_wallet_info(&wallet.wallet_id())
                .expect("the wallet")
                .identity_manager
                .wallet_identity(&wallet.wallet_id(), &owner())
                .expect("the claimant")
                .identity
                .clone()
        };
        let first_key_id = *claimant
            .public_keys()
            .keys()
            .next()
            .expect("the claimant's first key");
        let claim = ContractFeeClaimTransition::try_from_identity_with_signer(
            &claimant,
            &first_key_id,
            contract_id,
            ContractFeePot::Owner,
            STORED_NONCE + 1,
            0,
            &FixedSignatureSigner,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("the claim");
        let broadcast = claim
            .broadcast_request_for_state_transition()
            .expect("serializes");
        assert!(
            sdk.mock().request_count(&broadcast).await > 0,
            "the claim must broadcast the owner pot claim at nonce {}",
            STORED_NONCE + 1
        );
    }

    /// Every balance the proof of a claim shows for an identity this wallet holds is kept
    /// at the proof's block, and one of an identity it does not hold is ignored.
    #[tokio::test]
    async fn should_keep_the_proved_balance_of_each_wallet_identity_the_claim_paid() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        let claimed = ClaimedContractFees {
            contract_id: id(0xEE),
            pot: ContractFeePot::Moderators,
            last_claim: ContractFeePotLastClaim {
                epoch_index: 3,
                time_ms: 1_700_000_000_000,
                claimant_id: owner(),
            },
            remaining_credits: 1,
            balances: BTreeMap::from([(owner(), 777_000), (id(5), 555_000)]),
        };
        let metadata = ResponseMetadata {
            height: 42,
            core_chain_locked_height: 7,
            time_ms: 1_700_000_000_000,
            ..Default::default()
        };
        wallet
            .identity()
            .persist_claimed_balances(&claimed, &metadata)
            .await;

        let wm = wallet.identity().wallet_manager.read().await;
        let manager = &wm
            .get_wallet_info(&wallet.wallet_id())
            .expect("the wallet")
            .identity_manager;
        assert_eq!(
            manager
                .wallet_identity(&wallet.wallet_id(), &owner())
                .expect("the claimant")
                .identity
                .balance(),
            777_000
        );
        assert!(manager.identity(&id(5)).is_none());
    }

    /// Platform's refusal of a pot already paid out this epoch keeps its consensus error,
    /// so the FFI hands the host its code.
    #[test]
    fn should_keep_platforms_refusal_of_a_pot_already_claimed_this_epoch() {
        let cause =
            ConsensusError::StateError(StateError::ContractFeesAlreadyClaimedThisEpochError(
                ContractFeesAlreadyClaimedThisEpochError::new(id(0xEE), ContractFeePot::Owner, 3),
            ));
        let refusal =
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 41111,
                message: cause.to_string(),
                cause: Some(cause),
            });
        let error = PlatformWalletError::from(refusal);
        assert!(
            matches!(
                error.consensus_error(),
                Some(ConsensusError::StateError(
                    StateError::ContractFeesAlreadyClaimedThisEpochError(_)
                ))
            ),
            "{error:?}"
        );
    }
}
