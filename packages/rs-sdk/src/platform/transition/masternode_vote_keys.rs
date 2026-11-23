//! Voting-key facts for masternode contested-resource votes.
//!
//! Casting a vote needs three things that are properties of Platform, not of
//! any particular binding: which key a voter identity holds, how to recognise
//! that Platform rejected that key, and what the rejection actually meant.
//! They lived in `rs-sdk-ffi`, which meant Swift and Kotlin callers got them
//! (they route through the FFI) while Rust callers of [`PutVote`] did not, and
//! had to rediscover them — including the byte-order rule that made a vote
//! address an identity that never existed.
//!
//! Signing stays with the caller: producing a [`Signer`] from a raw key is a
//! binding concern, and pulling `simple-signer` into the public SDK to do it
//! here would be the wrong trade. [`put_masternode_vote`] takes the caller's
//! signer and owns the rest: the broadcast, and the diagnosis of a refused
//! voting key.
//!
//!
//! [`check_masternode_voting_key`] asks the same question before any vote: whether a
//! `(pro_tx_hash, voting address)` pair is a masternode voter Platform knows, so a host can
//! refuse a mismatched pair when it is entered rather than when a vote is refused.
//!
//! [`load_masternode_for_voting`] decides which identity a host stores to vote for a
//! masternode: its owner identity (at the pro_tx_hash) when Platform holds one, or else its
//! voter identity when Platform proves that one holds the enabled voting key. From protocol
//! version 14 a masternode without an owner address, such as a shared masternode, has no owner
//! identity, so the voter identity is the only one there is.
//!
//! [`PutVote`]: super::vote::PutVote
//! [`Signer`]: dpp::identity::signer::Signer

use dpp::dashcore::ProTxHash;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Identifier};

use dpp::identity::signer::Signer;
use dpp::voting::votes::Vote;

use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::vote::PutVote;
use crate::platform::Fetch;
use crate::{Error, Sdk};

/// Why Platform refused to accept a vote's voting key.
///
/// Both cases reach the caller as the same opaque "Public key 0 doesn't exist",
/// so they are only distinguishable by fetching the voter identity — see
/// [`diagnose_voting_key_failure`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VotingKeyProblem {
    /// No voter identity exists for this `(pro_tx_hash, voting address)` pair.
    /// Either `pro_tx_hash` is not a masternode, the voting key does not match
    /// the masternode's registered voting address, or Platform has not created
    /// the identity yet.
    NoVoterIdentity {
        /// The masternode the vote was cast for.
        pro_tx_hash: ProTxHash,
        /// The identity that would have held the key.
        expected_voter_identity: Identifier,
    },
    /// The voter identity exists but holds no enabled `ECDSA_HASH160` voting
    /// key for this address — what a rotation leaves behind, since
    /// `update_voter_identity_v0` disables the old identity's keys rather than
    /// removing them, and what a removed masternode leaves behind
    /// (`disable_identity_keys_v0`).
    NoUsableVotingKey {
        /// The identity that was fetched.
        voter_identity: Identifier,
    },
}

/// The voter identity id for a masternode's voting key.
///
/// Takes a typed [`ProTxHash`] rather than bytes on purpose: the derivation is
/// orientation-sensitive and `Txid` bytes for the same transaction are its
/// exact reverse (`ProTxHash` is `#[hash_newtype(forward)]`, `Txid` is not).
/// Passing the wrong one yields an identity that has never existed instead of
/// an error, so the type is the guard.
pub fn voter_identity_id(pro_tx_hash: ProTxHash, voting_address: &[u8; 20]) -> Identifier {
    Identifier::create_voter_identifier(&pro_tx_hash.to_byte_array(), voting_address)
}

/// The voting key Platform holds for a voter identity at `voting_address`.
///
/// Knowable without a round trip: `create_voter_identity_v0` assigns id 0, and
/// a rotation creates a *different* identity — the identifier includes the
/// voting address — whose key is likewise 0. So a caller can build this and
/// broadcast, rather than fetching the identity first.
pub fn voter_identity_voting_key(voting_address: &[u8; 20]) -> IdentityPublicKey {
    IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::VOTING,
        security_level: SecurityLevel::HIGH,
        key_type: KeyType::ECDSA_HASH160,
        read_only: true,
        data: BinaryData::new(voting_address.to_vec()),
        disabled_at: None,
        contract_bounds: None,
    }
    .into()
}

/// The usable voting key on a fetched voter identity, if any.
///
/// Matches on the key's own data rather than its position. Position would
/// usually work — Platform assigns id 0 — but it silently picks the wrong key
/// on an identity carrying others, and cannot tell a usable key from one a
/// rotation disabled. `disabled_at` is therefore part of the match, not an
/// afterthought: a disabled key exists and would be selected by id.
pub fn select_voting_key(
    identity: &Identity,
    voting_address: &[u8; 20],
) -> Option<IdentityPublicKey> {
    identity
        .public_keys()
        .values()
        .find(|key| {
            key.purpose() == Purpose::VOTING
                && key.key_type() == KeyType::ECDSA_HASH160
                && key.data().as_slice() == voting_address
                && key.disabled_at().is_none()
        })
        .cloned()
}

/// Whether a failed vote broadcast is Platform rejecting the VOTING KEY, as
/// opposed to anything else that can fail a vote.
///
/// Matched on the typed consensus error rather than its rendered text: these
/// three signature variants are exactly what [`diagnose_voting_key_failure`]
/// can explain, and a message match would silently begin diagnosing unrelated
/// failures the first time a string changed.
///
/// Callers should gate diagnosis on this. Diagnosing every failure lets an
/// absent voter identity masquerade as the cause of a closed poll or a
/// transport error.
pub fn is_voting_key_failure(error: &Error) -> bool {
    use dpp::consensus::signature::SignatureError;
    use dpp::consensus::ConsensusError;

    fn is_key_signature_error(consensus: &ConsensusError) -> bool {
        matches!(
            consensus,
            ConsensusError::SignatureError(
                SignatureError::IdentityNotFoundError(_)
                    | SignatureError::MissingPublicKeyError(_)
                    | SignatureError::PublicKeyIsDisabledError(_)
            )
        )
    }

    match error {
        // Rejected at broadcast: the consensus error rides on the response.
        Error::StateTransitionBroadcastError(e) => {
            e.cause.as_ref().is_some_and(is_key_signature_error)
        }
        // Rejected locally / surfaced as a protocol error.
        Error::Protocol(dpp::ProtocolError::ConsensusError(e)) => is_key_signature_error(e),
        _ => false,
    }
}

/// What Platform's state proves about a masternode voting key.
///
/// Platform creates the voter identity `voter_identity_id(pro_tx_hash, voting_address)` only
/// for a masternode in its masternode list with that registered voting address, gives it the
/// voting key, and disables that key when the masternode is removed or its voting address
/// changes. So one proof-verified fetch of that identity, with no Core or SPV sync, proves
/// whether Platform holds an enabled voter key for the pair.
///
/// That is all it proves: the fetch does not read the masternode list, which a vote is checked
/// against. One history makes the two differ. Before protocol version 15
/// (`update_voter_identity_v0`), a masternode that moved its voting address from A to B and
/// back to A was left with A's key disabled and B's key enabled. B then reads as
/// [`VotingKeyCheck::Valid`], but a vote with it is refused with
/// `MasternodeIncorrectVotingAddressError`, since the list names A; A reads as
/// [`VotingKeyCheck::Invalid`] and cannot vote either. Telling B apart needs the registered
/// voting address from Core.
#[derive(Debug)]
pub enum VotingKeyCheck {
    /// The voter identity exists and holds an enabled voting key for this address. Apart from
    /// the history described on [`VotingKeyCheck`], the key can vote for this masternode. A
    /// host stores it as checked.
    Valid {
        /// The voter identity that holds the key.
        voter_identity: Identifier,
    },
    /// Platform proves the key cannot vote for this masternode: the pro_tx_hash is not a
    /// masternode, the address is not its voting address, or the key was rotated away or
    /// disabled. A host refuses the key.
    Invalid(VotingKeyProblem),
    /// The voter identity could not be fetched (network, proof or SDK failure), so nothing is
    /// known about the key. A host may keep the key and mark it unchecked, to check again
    /// later.
    Undetermined(Box<Error>),
}

/// The verdict for the voter identity `fetched` from Platform at
/// `voter_identity_id(pro_tx_hash, voting_address)`: `None` when Platform proved it absent.
///
/// Kept apart from the fetch so the rule can be tested without a network. Private because it
/// trusts that `fetched` is the identity at that id, which only the proved fetch in
/// [`check_masternode_voting_key`] guarantees.
fn voting_key_check_from_voter_identity(
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
    fetched: Option<&Identity>,
) -> Result<Identifier, VotingKeyProblem> {
    let expected_voter_identity = voter_identity_id(pro_tx_hash, voting_address);
    let Some(identity) = fetched else {
        return Err(VotingKeyProblem::NoVoterIdentity {
            pro_tx_hash,
            expected_voter_identity,
        });
    };
    match select_voting_key(identity, voting_address) {
        Some(_) => Ok(expected_voter_identity),
        None => Err(VotingKeyProblem::NoUsableVotingKey {
            voter_identity: expected_voter_identity,
        }),
    }
}

/// Checks whether Platform holds an enabled voter key with `voting_address` for the masternode
/// `pro_tx_hash`, by one proof-verified fetch of its voter identity. See [`VotingKeyCheck`] for
/// what that does and does not prove.
///
/// A masternode registered, or a voting key changed, in the last few blocks reads as
/// [`VotingKeyCheck::Invalid`] until Platform processes the masternode list update.
pub async fn check_masternode_voting_key(
    sdk: &Sdk,
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
) -> VotingKeyCheck {
    let fetched = Identity::fetch(sdk, voter_identity_id(pro_tx_hash, voting_address)).await;
    voting_key_check(pro_tx_hash, voting_address, fetched)
}

/// The identity a host stores to vote for a masternode, as
/// [`load_masternode_for_voting`] found it.
#[derive(Debug)]
pub enum MasternodeVotingIdentity {
    /// The masternode's owner identity, whose id is the pro_tx_hash. Every masternode has one
    /// before protocol version 14, and from 14 every masternode with an owner address.
    Owner(Identity),
    /// The masternode's voter identity (`voter_identity_id(pro_tx_hash, voting_address)`),
    /// holding the enabled voting key. Loaded only when Platform proves the owner identity
    /// absent: from protocol version 14 `create_owner_identity` v2 creates none for a
    /// masternode without an owner address, such as a shared masternode, while its voter
    /// identity still exists. Its id is not the pro_tx_hash, so a host keeps the pro_tx_hash
    /// apart from it.
    Voter(Identity),
}

impl MasternodeVotingIdentity {
    /// The fetched identity.
    pub fn identity(&self) -> &Identity {
        match self {
            MasternodeVotingIdentity::Owner(identity)
            | MasternodeVotingIdentity::Voter(identity) => identity,
        }
    }
}

/// Why [`load_masternode_for_voting`] loads nothing for a masternode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasternodeVotingLoadRefusal {
    /// Platform proves no owner identity exists at the pro_tx_hash, and no voting key was
    /// given, so no voter identity can be named: either the pro_tx_hash is not a masternode,
    /// or the masternode has no owner identity (no owner address, from protocol version 14)
    /// and can be loaded only with its voting key.
    NoOwnerIdentity {
        /// The masternode that was asked for.
        pro_tx_hash: ProTxHash,
    },
    /// Platform proves no owner identity exists at the pro_tx_hash, and the voting key cannot
    /// vote for it either.
    NoOwnerIdentityAndUnusableVotingKey {
        /// The masternode that was asked for.
        pro_tx_hash: ProTxHash,
        /// Why the voting key cannot vote.
        problem: VotingKeyProblem,
    },
}

/// What [`load_masternode_for_voting`] decided.
#[derive(Debug)]
pub enum MasternodeVotingLoad {
    /// Store `identity` as the masternode's row, with the pro_tx_hash kept beside it.
    Load {
        /// The identity to store, owner or voter.
        identity: MasternodeVotingIdentity,
        /// The check of the voting key, when one was given. Always
        /// [`VotingKeyCheck::Valid`] for [`MasternodeVotingIdentity::Voter`]. For the owner
        /// identity it is what [`check_masternode_voting_key`] returns, and the host applies
        /// [`voting_key_check_outcome`] to it as before: an invalid key is refused, an
        /// undetermined one stored unchecked.
        voting_key: Option<VotingKeyCheck>,
    },
    /// Load nothing.
    Refused(MasternodeVotingLoadRefusal),
}

/// A voting address with the result of fetching its voter identity.
type FetchedVoter<'a> = (&'a [u8; 20], Result<Option<Identity>, Error>);

/// The rule of [`load_masternode_for_voting`], from what the fetches returned: `owner` is the
/// identity Platform proved at the pro_tx_hash (`None`: proved absent), and `voter`, present
/// exactly when a voting key was given, its voting address with the result of fetching its
/// voter identity.
///
/// Kept apart from the fetches so the rule can be tested without a network. Private because it
/// trusts that each identity is the one at its id, which only the proved fetches guarantee.
fn masternode_voting_load(
    pro_tx_hash: ProTxHash,
    owner: Option<Identity>,
    voter: Option<FetchedVoter<'_>>,
) -> Result<MasternodeVotingLoad, Error> {
    let Some((voting_address, fetched_voter)) = voter else {
        return Ok(match owner {
            Some(owner) => MasternodeVotingLoad::Load {
                identity: MasternodeVotingIdentity::Owner(owner),
                voting_key: None,
            },
            None => MasternodeVotingLoad::Refused(MasternodeVotingLoadRefusal::NoOwnerIdentity {
                pro_tx_hash,
            }),
        });
    };
    if let Some(owner) = owner {
        // A masternode with an owner identity loads as before, whatever the voting key.
        return Ok(MasternodeVotingLoad::Load {
            identity: MasternodeVotingIdentity::Owner(owner),
            voting_key: Some(voting_key_check(pro_tx_hash, voting_address, fetched_voter)),
        });
    }
    // No owner identity: only a voter identity Platform proves to hold the enabled voting key
    // is loaded. A voter fetch that failed proves nothing, so nothing can be stored.
    let checked = match fetched_voter? {
        Some(identity) => {
            voting_key_check_from_voter_identity(pro_tx_hash, voting_address, Some(&identity))
                .map(|voter_identity| (identity, voter_identity))
        }
        None => Err(VotingKeyProblem::NoVoterIdentity {
            pro_tx_hash,
            expected_voter_identity: voter_identity_id(pro_tx_hash, voting_address),
        }),
    };
    Ok(match checked {
        Ok((identity, voter_identity)) => MasternodeVotingLoad::Load {
            identity: MasternodeVotingIdentity::Voter(identity),
            voting_key: Some(VotingKeyCheck::Valid { voter_identity }),
        },
        Err(problem) => MasternodeVotingLoad::Refused(
            MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
                pro_tx_hash,
                problem,
            },
        ),
    })
}

/// The [`VotingKeyCheck`] for the result of fetching the voter identity of
/// `(pro_tx_hash, voting_address)`.
fn voting_key_check(
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
    fetched: Result<Option<Identity>, Error>,
) -> VotingKeyCheck {
    let fetched = match fetched {
        Ok(fetched) => fetched,
        Err(error) => return VotingKeyCheck::Undetermined(Box::new(error)),
    };
    match voting_key_check_from_voter_identity(pro_tx_hash, voting_address, fetched.as_ref()) {
        Ok(voter_identity) => VotingKeyCheck::Valid { voter_identity },
        Err(problem) => VotingKeyCheck::Invalid(problem),
    }
}

/// Which identity a host loads to vote for the masternode `pro_tx_hash`, with the voting key
/// of `voting_address` when the user gave one.
///
/// Fetches, proof-verified and side by side, the owner identity at the pro_tx_hash and, with a
/// voting key, the voter identity of `(pro_tx_hash, voting_address)`:
///
/// - Owner identity present: it is loaded, as every masternode was loaded before, with the
///   voting key's [`VotingKeyCheck`] for the host's existing rules.
/// - Owner identity proved absent, voter identity proved present with the enabled voting key:
///   the voter identity is loaded. This is a masternode with no owner address (a shared
///   masternode), which has no owner identity from protocol version 14.
/// - Owner identity proved absent otherwise: refused ([`MasternodeVotingLoadRefusal`]).
///
/// The host stores the pro_tx_hash apart from the loaded identity's id: for a voter identity
/// the two differ, and votes name the masternode by its pro_tx_hash.
///
/// # Errors
/// The owner identity could not be fetched, or it is absent and the voter identity could not
/// be fetched: nothing is proved, so nothing is loaded.
pub async fn load_masternode_for_voting(
    sdk: &Sdk,
    pro_tx_hash: ProTxHash,
    voting_address: Option<&[u8; 20]>,
) -> Result<MasternodeVotingLoad, Error> {
    let owner = Identity::fetch(sdk, Identifier::new(pro_tx_hash.to_byte_array()));
    let voter = async {
        match voting_address {
            Some(address) => Some((
                address,
                Identity::fetch(sdk, voter_identity_id(pro_tx_hash, address)).await,
            )),
            None => None,
        }
    };
    let (owner, voter) = futures::join!(owner, voter);
    masternode_voting_load(pro_tx_hash, owner?, voter)
}

/// A [`VotingKeyCheck`] without its details: what a host records for a stored voting key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedVotingKeyCheck {
    /// The check was [`VotingKeyCheck::Valid`].
    Valid,
    /// The check was [`VotingKeyCheck::Invalid`].
    Invalid,
    /// The check was [`VotingKeyCheck::Undetermined`].
    Undetermined,
}

impl VotingKeyCheck {
    /// What a host records for this check.
    pub fn recorded(&self) -> RecordedVotingKeyCheck {
        match self {
            VotingKeyCheck::Valid { .. } => RecordedVotingKeyCheck::Valid,
            VotingKeyCheck::Invalid(_) => RecordedVotingKeyCheck::Invalid,
            VotingKeyCheck::Undetermined(_) => RecordedVotingKeyCheck::Undetermined,
        }
    }
}

/// A check a host recorded, with the voting address of the key it was made of.
///
/// A host's key store and its records are written separately, so a record can outlive the key
/// it describes (a replacement whose key was written but whose record was not). The address
/// ties the record to its key: the rules here use a record only for a key with that address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedVotingKey {
    /// What the check found.
    pub check: RecordedVotingKeyCheck,
    /// The voting address of the key that was checked.
    pub voting_address: [u8; 20],
}

impl RecordedVotingKey {
    /// The recorded check, if it was made of the key with `voting_address`.
    fn check_of(&self, voting_address: &[u8; 20]) -> Option<RecordedVotingKeyCheck> {
        (self.voting_address == *voting_address).then_some(self.check)
    }
}

/// What the host's voting key slot for the masternode holds, compared with the key that was
/// checked. The host reads its own store to say this; the rules in
/// [`voting_key_check_outcome`] decide what follows from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VotingKeySlot {
    /// No voting key is stored for the masternode.
    Empty,
    /// Exactly the checked key is stored, with the record the host holds for the masternode,
    /// if any. The record counts only if it was made of this key (its voting address).
    SameKey(Option<RecordedVotingKey>),
    /// A different key is stored. The checked key replaces it only if the user confirms.
    DifferentKey,
}

/// What a host does with a voting key it was given, once checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VotingKeyCheckOutcome {
    /// Whether the host stores the checked key: every check but a proven refusal.
    pub store_key: bool,
    /// What the host records for the checked key once it is in the slot, or for the stored key
    /// when it is the same one: always made of the checked key, so it carries that key's voting
    /// address and the host persists it as one value. `None` leaves what is recorded unchanged:
    /// a check that could not run never erases what Platform proved about the same key before.
    pub record: Option<RecordedVotingKey>,
}

/// What a host does with a voting key after checking it ([`check_masternode_voting_key`]),
/// given what its voting key slot holds.
///
/// - Invalid: the key is not stored. If the slot already holds that same key, the refusal is
///   recorded on it, so the host stops offering it; a refusal of a different key changes
///   nothing stored.
/// - Valid: the key is stored and recorded as valid.
/// - Undetermined: the key is stored and recorded as undetermined, unless the slot already
///   holds the same key with a valid or invalid record of that key, which is kept.
///
/// A record made of another key (another voting address) is not a record of the checked key,
/// so it is never kept for it.
pub fn voting_key_check_outcome(
    check: RecordedVotingKeyCheck,
    voting_address: &[u8; 20],
    slot: VotingKeySlot,
) -> VotingKeyCheckOutcome {
    let slot = match slot {
        VotingKeySlot::SameKey(recorded) => VotingKeySlot::SameKey(
            recorded.filter(|recorded| recorded.check_of(voting_address).is_some()),
        ),
        other => other,
    };
    let (store_key, record) = match check {
        RecordedVotingKeyCheck::Invalid => (
            false,
            match slot {
                VotingKeySlot::SameKey(_) => Some(RecordedVotingKeyCheck::Invalid),
                VotingKeySlot::Empty | VotingKeySlot::DifferentKey => None,
            },
        ),
        RecordedVotingKeyCheck::Valid => (true, Some(RecordedVotingKeyCheck::Valid)),
        RecordedVotingKeyCheck::Undetermined => (
            true,
            match slot {
                VotingKeySlot::SameKey(Some(RecordedVotingKey {
                    check: RecordedVotingKeyCheck::Valid | RecordedVotingKeyCheck::Invalid,
                    ..
                })) => None,
                VotingKeySlot::SameKey(_) | VotingKeySlot::Empty | VotingKeySlot::DifferentKey => {
                    Some(RecordedVotingKeyCheck::Undetermined)
                }
            },
        ),
    };
    VotingKeyCheckOutcome {
        store_key,
        record: record.map(|check| RecordedVotingKey {
            check,
            voting_address: *voting_address,
        }),
    }
}

/// How a host may use a stored voting key, from what it recorded for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedVotingKeyUse {
    /// Whether the key is offered for voting: every record but a proven refusal.
    pub can_vote: bool,
    /// Whether the host checks the key ([`check_masternode_voting_key`]) before using it: when
    /// it was never checked (`None`, such as a key stored before checks existed) or the last
    /// check could not run.
    pub check_before_use: bool,
}

/// How a host may use a stored voting key, from the record it holds for the masternode.
///
/// `stored_voting_address` is the voting address of the key in the slot, when the host has
/// read it. A record made of another key does not describe it, so the key is then treated as
/// never checked. Without the address (a host listing many keys without reading each), the
/// record is taken as it is, and the check before use re-reads it.
pub fn recorded_voting_key_use(
    recorded: Option<RecordedVotingKey>,
    stored_voting_address: Option<&[u8; 20]>,
) -> RecordedVotingKeyUse {
    let recorded = recorded.and_then(|recorded| match stored_voting_address {
        Some(address) => recorded.check_of(address),
        None => Some(recorded.check),
    });
    match recorded {
        Some(RecordedVotingKeyCheck::Valid) => RecordedVotingKeyUse {
            can_vote: true,
            check_before_use: false,
        },
        Some(RecordedVotingKeyCheck::Invalid) => RecordedVotingKeyUse {
            can_vote: false,
            check_before_use: false,
        },
        Some(RecordedVotingKeyCheck::Undetermined) | None => RecordedVotingKeyUse {
            can_vote: true,
            check_before_use: true,
        },
    }
}

/// Explain a voting-key rejection by fetching the voter identity.
///
/// Costs one Platform round trip, so gate it on [`is_voting_key_failure`] and
/// run it only after a broadcast has already failed — doing it before every
/// cast spends a round trip per (masternode, contest) on runs that
/// overwhelmingly succeed.
///
/// Returns `None` when the identity and its key both check out, or when the
/// fetch itself fails: neither says anything, so the caller keeps its original
/// error rather than replacing it with a guess.
pub async fn diagnose_voting_key_failure(
    sdk: &Sdk,
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
) -> Option<VotingKeyProblem> {
    match check_masternode_voting_key(sdk, pro_tx_hash, voting_address).await {
        VotingKeyCheck::Invalid(problem) => Some(problem),
        VotingKeyCheck::Valid { .. } | VotingKeyCheck::Undetermined(_) => None,
    }
}

/// Why a masternode vote was not recorded.
#[derive(Debug)]
pub enum MasternodeVoteFailure {
    /// Platform refused the voting key, and the voter identity says why.
    VotingKey(VotingKeyProblem),
    /// Any other failure, or a refused voting key the voter identity does not
    /// explain.
    Broadcast(Box<Error>),
}

/// Signs `vote` for the masternode `pro_tx_hash` with the voting key of
/// `voting_address` that `signer` holds, broadcasts it, and waits for the vote
/// Platform recorded.
///
/// The voting key is the one Platform gives every voter identity
/// ([`voter_identity_voting_key`]), so nothing is fetched before the
/// broadcast. Only when Platform refuses that key
/// ([`is_voting_key_failure`]) is the voter identity fetched to say why
/// ([`diagnose_voting_key_failure`]); every other failure, and a refusal the
/// identity does not explain, comes back as it was.
pub async fn put_masternode_vote<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    vote: &Vote,
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<Vote, MasternodeVoteFailure> {
    let voting_key = voter_identity_voting_key(voting_address);
    let broadcast_error = match vote
        .put_to_platform_and_wait_for_response(
            Identifier::new(pro_tx_hash.to_byte_array()),
            &voting_key,
            sdk,
            signer,
            settings,
        )
        .await
    {
        Ok(recorded) => return Ok(recorded),
        Err(error) => error,
    };
    if !is_voting_key_failure(&broadcast_error) {
        return Err(MasternodeVoteFailure::Broadcast(Box::new(broadcast_error)));
    }
    match diagnose_voting_key_failure(sdk, pro_tx_hash, voting_address).await {
        Some(problem) => Err(MasternodeVoteFailure::VotingKey(problem)),
        None => Err(MasternodeVoteFailure::Broadcast(Box::new(broadcast_error))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::signature::{
        BasicECDSAError, IdentityNotFoundError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    };
    use dpp::consensus::ConsensusError;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

    const VOTING_ADDRESS: [u8; 20] = [7u8; 20];
    const OTHER_ADDRESS: [u8; 20] = [9u8; 20];

    fn identity_id() -> Identifier {
        Identifier::new([3u8; 32])
    }

    fn key(
        id: u32,
        purpose: Purpose,
        key_type: KeyType,
        data: [u8; 20],
        disabled_at: Option<u64>,
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(data.to_vec()),
            disabled_at,
        })
    }

    fn identity_with(keys: Vec<IdentityPublicKey>) -> Identity {
        let mut public_keys = BTreeMap::new();
        for k in keys {
            public_keys.insert(k.id(), k);
        }
        Identity::V0(IdentityV0 {
            id: identity_id(),
            public_keys,
            balance: 0,
            revision: 0,
        })
    }

    fn broadcast_error_with(cause: ConsensusError) -> Error {
        Error::StateTransitionBroadcastError(crate::error::StateTransitionBroadcastError {
            code: 1,
            message: "rejected".to_string(),
            cause: Some(cause),
        })
    }

    /// The fabricated key must be exactly what Platform holds, or broadcasting
    /// with it fails for a reason that looks like a missing identity.
    #[test]
    fn fabricated_key_matches_what_platform_assigns() {
        let key = voter_identity_voting_key(&VOTING_ADDRESS);
        assert_eq!(key.id(), 0);
        assert_eq!(key.purpose(), Purpose::VOTING);
        assert_eq!(key.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(key.data().as_slice(), &VOTING_ADDRESS);
        assert!(key.disabled_at().is_none());
    }

    /// The identity id is orientation-sensitive: the same 32 bytes reversed
    /// address a different identity. This is the property that made a vote fail
    /// as "no voter identity" when callers passed `Txid` bytes.
    #[test]
    fn voter_identity_id_is_orientation_sensitive() {
        let bytes = [1u8; 32];
        let mut reversed = bytes;
        reversed.reverse();
        // Distinct bytes so the two orientations cannot coincide.
        let mut asymmetric = [0u8; 32];
        asymmetric[0] = 1;

        let forward = voter_identity_id(ProTxHash::from_byte_array(asymmetric), &VOTING_ADDRESS);
        let mut flipped = asymmetric;
        flipped.reverse();
        let backward = voter_identity_id(ProTxHash::from_byte_array(flipped), &VOTING_ADDRESS);

        assert_ne!(
            forward, backward,
            "reversing the pro_tx_hash must change the identity — the type is \
             the only thing keeping callers honest"
        );
    }

    #[test]
    fn selects_the_enabled_voting_key() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_some());
    }

    /// Selection is by data, not position: a voting key at a non-zero id behind
    /// another key at 0 must still be found.
    #[test]
    fn selects_by_data_not_by_position() {
        let identity = identity_with(vec![
            key(
                0,
                Purpose::AUTHENTICATION,
                KeyType::ECDSA_HASH160,
                OTHER_ADDRESS,
                None,
            ),
            key(
                4,
                Purpose::VOTING,
                KeyType::ECDSA_HASH160,
                VOTING_ADDRESS,
                None,
            ),
        ]);
        assert_eq!(
            select_voting_key(&identity, &VOTING_ADDRESS).map(|k| k.id()),
            Some(4)
        );
    }

    /// What a rotation leaves behind — the key exists and would be picked by id.
    #[test]
    fn rejects_a_disabled_voting_key() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            Some(1_700_000_000),
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    #[test]
    fn rejects_a_different_address() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            OTHER_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    /// Purpose must be load-bearing on its own: the same address and key type
    /// under AUTHENTICATION must not be selected.
    #[test]
    fn rejects_a_matching_address_under_the_wrong_purpose() {
        let identity = identity_with(vec![key(
            0,
            Purpose::AUTHENTICATION,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    #[test]
    fn rejects_a_matching_address_under_the_wrong_key_type() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::BIP13_SCRIPT_HASH,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    // ---- checking a (pro_tx_hash, voting address) pair ----------------------

    /// A pro_tx_hash whose two byte orders differ, so a reversal would show.
    fn pro_tx_hash() -> ProTxHash {
        ProTxHash::from_byte_array(std::array::from_fn(|i| i as u8 + 1))
    }

    /// The voter identity Platform creates for `pro_tx_hash` and `voting_address`: its own id,
    /// not the masternode's (owner) identity id, holding the voting key at id 0.
    fn voter_identity(
        pro_tx_hash: ProTxHash,
        voting_address: [u8; 20],
        disabled_at: Option<u64>,
    ) -> Identity {
        let mut public_keys = BTreeMap::new();
        public_keys.insert(
            0,
            key(
                0,
                Purpose::VOTING,
                KeyType::ECDSA_HASH160,
                voting_address,
                disabled_at,
            ),
        );
        Identity::V0(IdentityV0 {
            id: voter_identity_id(pro_tx_hash, &voting_address),
            public_keys,
            balance: 0,
            revision: 0,
        })
    }

    /// The masternode's identity is named by its pro_tx_hash; its voter identity is a
    /// different id, derived from the pro_tx_hash and the voting address together.
    #[test]
    fn should_derive_a_voter_identity_distinct_from_the_owner_identity() {
        let owner_identity = Identifier::new(pro_tx_hash().to_byte_array());
        let voter = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        assert_ne!(voter, owner_identity);
        assert_ne!(voter, voter_identity_id(pro_tx_hash(), &OTHER_ADDRESS));
    }

    #[test]
    fn should_accept_a_voter_identity_holding_the_enabled_voting_key() {
        let identity = voter_identity(pro_tx_hash(), VOTING_ADDRESS, None);
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &VOTING_ADDRESS, Some(&identity)),
            Ok(voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS))
        );
    }

    /// Absent: not a masternode, or not its voting address. The problem names the voter
    /// identity that was looked for, not the masternode's own identity.
    #[test]
    fn should_refuse_a_pair_with_no_voter_identity() {
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &VOTING_ADDRESS, None),
            Err(VotingKeyProblem::NoVoterIdentity {
                pro_tx_hash: pro_tx_hash(),
                expected_voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
            })
        );
    }

    /// What a removed masternode or a rotated voting key leaves behind.
    #[test]
    fn should_refuse_a_voter_identity_whose_voting_key_is_disabled() {
        let identity = voter_identity(pro_tx_hash(), VOTING_ADDRESS, Some(1_700_000_000));
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &VOTING_ADDRESS, Some(&identity)),
            Err(VotingKeyProblem::NoUsableVotingKey {
                voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
            })
        );
    }

    /// The history `update_voter_identity_v0` leaves after a masternode moves its voting
    /// address from A to B and back to A: A's key disabled, B's enabled. The proof can only
    /// say so: B reads as valid although the masternode list names A (see [`VotingKeyCheck`]),
    /// and A reads as invalid. If this changes, update the documented limit.
    #[test]
    fn should_report_what_a_pre_v15_return_to_a_voting_address_leaves_behind() {
        let address_a = VOTING_ADDRESS;
        let address_b = OTHER_ADDRESS;
        let identity_a = voter_identity(pro_tx_hash(), address_a, Some(1_700_000_000));
        let identity_b = voter_identity(pro_tx_hash(), address_b, None);

        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &address_b, Some(&identity_b)),
            Ok(voter_identity_id(pro_tx_hash(), &address_b))
        );
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &address_a, Some(&identity_a)),
            Err(VotingKeyProblem::NoUsableVotingKey {
                voter_identity: voter_identity_id(pro_tx_hash(), &address_a),
            })
        );
    }

    // ---- which identity a host loads to vote --------------------------------

    /// The masternode's owner identity: its id is the pro_tx_hash.
    fn owner_identity() -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::new(pro_tx_hash().to_byte_array()),
            public_keys: BTreeMap::new(),
            balance: 5,
            revision: 0,
        })
    }

    fn loaded(
        load: Result<MasternodeVotingLoad, Error>,
    ) -> (MasternodeVotingIdentity, Option<VotingKeyCheck>) {
        match load {
            Ok(MasternodeVotingLoad::Load {
                identity,
                voting_key,
            }) => (identity, voting_key),
            other => panic!("expected a load, got {other:?}"),
        }
    }

    fn refused(load: Result<MasternodeVotingLoad, Error>) -> MasternodeVotingLoadRefusal {
        match load {
            Ok(MasternodeVotingLoad::Refused(refusal)) => refusal,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// A masternode with an owner identity loads it, as before, with no voting key too.
    #[test]
    fn should_load_the_owner_identity_without_a_voting_key() {
        let (identity, voting_key) = loaded(masternode_voting_load(
            pro_tx_hash(),
            Some(owner_identity()),
            None,
        ));
        assert!(matches!(identity, MasternodeVotingIdentity::Owner(_)));
        assert_eq!(identity.identity().id(), owner_identity().id());
        assert!(voting_key.is_none());
    }

    /// With an owner identity, the voting key's check comes back for the host's existing
    /// rules: valid, invalid or undetermined, never a refusal of the load itself.
    #[test]
    fn should_load_the_owner_identity_with_the_voting_key_check_as_before() {
        let (identity, voting_key) = loaded(masternode_voting_load(
            pro_tx_hash(),
            Some(owner_identity()),
            Some((
                &VOTING_ADDRESS,
                Ok(Some(voter_identity(pro_tx_hash(), VOTING_ADDRESS, None))),
            )),
        ));
        assert!(matches!(identity, MasternodeVotingIdentity::Owner(_)));
        assert!(matches!(voting_key, Some(VotingKeyCheck::Valid { .. })));

        let (identity, voting_key) = loaded(masternode_voting_load(
            pro_tx_hash(),
            Some(owner_identity()),
            Some((&VOTING_ADDRESS, Ok(None))),
        ));
        assert!(matches!(identity, MasternodeVotingIdentity::Owner(_)));
        assert!(matches!(
            voting_key,
            Some(VotingKeyCheck::Invalid(
                VotingKeyProblem::NoVoterIdentity { .. }
            ))
        ));

        let (identity, voting_key) = loaded(masternode_voting_load(
            pro_tx_hash(),
            Some(owner_identity()),
            Some((&VOTING_ADDRESS, Err(Error::Generic("timeout".to_string())))),
        ));
        assert!(matches!(identity, MasternodeVotingIdentity::Owner(_)));
        assert!(matches!(voting_key, Some(VotingKeyCheck::Undetermined(_))));
    }

    /// A shared masternode: no owner identity, and a voter identity holding the enabled key.
    #[test]
    fn should_load_the_voter_identity_when_the_owner_is_absent_and_the_voter_holds_the_key() {
        let (identity, voting_key) = loaded(masternode_voting_load(
            pro_tx_hash(),
            None,
            Some((
                &VOTING_ADDRESS,
                Ok(Some(voter_identity(pro_tx_hash(), VOTING_ADDRESS, None))),
            )),
        ));
        assert!(matches!(identity, MasternodeVotingIdentity::Voter(_)));
        assert_eq!(
            identity.identity().id(),
            voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS)
        );
        match voting_key {
            Some(VotingKeyCheck::Valid { voter_identity }) => assert_eq!(
                voter_identity,
                voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS)
            ),
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn should_refuse_when_the_owner_and_the_voter_identity_are_absent() {
        assert_eq!(
            refused(masternode_voting_load(
                pro_tx_hash(),
                None,
                Some((&VOTING_ADDRESS, Ok(None)))
            )),
            MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
                pro_tx_hash: pro_tx_hash(),
                problem: VotingKeyProblem::NoVoterIdentity {
                    pro_tx_hash: pro_tx_hash(),
                    expected_voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
                },
            }
        );
    }

    #[test]
    fn should_refuse_when_the_owner_is_absent_and_the_voting_key_is_disabled() {
        assert_eq!(
            refused(masternode_voting_load(
                pro_tx_hash(),
                None,
                Some((
                    &VOTING_ADDRESS,
                    Ok(Some(voter_identity(
                        pro_tx_hash(),
                        VOTING_ADDRESS,
                        Some(1_700_000_000)
                    ))),
                )),
            )),
            MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
                pro_tx_hash: pro_tx_hash(),
                problem: VotingKeyProblem::NoUsableVotingKey {
                    voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
                },
            }
        );
    }

    /// Without a voting key no voter identity can be named, so a masternode without an owner
    /// identity is refused, as it was before.
    #[test]
    fn should_refuse_a_masternode_without_an_owner_identity_or_a_voting_key() {
        assert_eq!(
            refused(masternode_voting_load(pro_tx_hash(), None, None)),
            MasternodeVotingLoadRefusal::NoOwnerIdentity {
                pro_tx_hash: pro_tx_hash()
            }
        );
    }

    /// With no owner identity, a voter fetch that failed proves nothing, so nothing loads.
    #[test]
    fn should_fail_when_the_owner_is_absent_and_the_voter_cannot_be_fetched() {
        assert!(matches!(
            masternode_voting_load(
                pro_tx_hash(),
                None,
                Some((&VOTING_ADDRESS, Err(Error::Generic("timeout".to_string())))),
            ),
            Err(Error::Generic(_))
        ));
    }

    // ---- what a host does with a checked key --------------------------------

    use RecordedVotingKeyCheck::{
        Invalid as RecordedInvalid, Undetermined as RecordedUndetermined, Valid as RecordedValid,
    };

    /// The outcome for the checked key (`VOTING_ADDRESS`): a record is always made of it.
    fn outcome(store_key: bool, record: Option<RecordedVotingKeyCheck>) -> VotingKeyCheckOutcome {
        VotingKeyCheckOutcome {
            store_key,
            record: record.map(|check| RecordedVotingKey {
                check,
                voting_address: VOTING_ADDRESS,
            }),
        }
    }

    /// A record of the key with `address`.
    fn record(check: RecordedVotingKeyCheck, address: [u8; 20]) -> Option<RecordedVotingKey> {
        Some(RecordedVotingKey {
            check,
            voting_address: address,
        })
    }

    /// Every slot a host can report for the key with `VOTING_ADDRESS`: the same key under each
    /// record of it, and under a record left by another key (`OTHER_ADDRESS`).
    fn every_slot() -> Vec<VotingKeySlot> {
        let mut slots = vec![
            VotingKeySlot::Empty,
            VotingKeySlot::DifferentKey,
            VotingKeySlot::SameKey(None),
        ];
        for check in [RecordedValid, RecordedInvalid, RecordedUndetermined] {
            slots.push(VotingKeySlot::SameKey(record(check, VOTING_ADDRESS)));
            slots.push(VotingKeySlot::SameKey(record(check, OTHER_ADDRESS)));
        }
        slots
    }

    /// Whether `slot` holds the checked key with a valid or invalid record made of it.
    fn holds_a_proven_record_of_the_checked_key(slot: VotingKeySlot) -> bool {
        matches!(
            slot,
            VotingKeySlot::SameKey(Some(RecordedVotingKey {
                check: RecordedValid | RecordedInvalid,
                voting_address,
            })) if voting_address == VOTING_ADDRESS
        )
    }

    #[test]
    fn should_record_what_each_check_is() {
        assert_eq!(
            VotingKeyCheck::Valid {
                voter_identity: identity_id()
            }
            .recorded(),
            RecordedValid
        );
        assert_eq!(
            VotingKeyCheck::Invalid(VotingKeyProblem::NoUsableVotingKey {
                voter_identity: identity_id()
            })
            .recorded(),
            RecordedInvalid
        );
        assert_eq!(
            VotingKeyCheck::Undetermined(Box::new(Error::Generic("timeout".to_string())))
                .recorded(),
            RecordedUndetermined
        );
    }

    /// A refused key is never stored; the refusal is recorded only on the same stored key.
    #[test]
    fn should_refuse_an_invalid_key_and_record_it_only_on_the_same_stored_key() {
        for slot in every_slot() {
            let expected = match slot {
                VotingKeySlot::SameKey(_) => outcome(false, Some(RecordedInvalid)),
                _ => outcome(false, None),
            };
            assert_eq!(
                voting_key_check_outcome(RecordedInvalid, &VOTING_ADDRESS, slot),
                expected,
                "{slot:?}"
            );
        }
    }

    #[test]
    fn should_store_and_record_a_valid_key_in_every_slot() {
        for slot in every_slot() {
            assert_eq!(
                voting_key_check_outcome(RecordedValid, &VOTING_ADDRESS, slot),
                outcome(true, Some(RecordedValid)),
                "{slot:?}"
            );
        }
    }

    /// A check that could not run stores the key, and does not erase a proven record of the
    /// same key. A record left by another key is not kept for it: a replacement whose key was
    /// written but whose record was not must not inherit the old key's refusal.
    #[test]
    fn should_store_an_undetermined_key_and_keep_only_a_proven_record_of_that_key() {
        for slot in every_slot() {
            let expected = if holds_a_proven_record_of_the_checked_key(slot) {
                outcome(true, None)
            } else {
                outcome(true, Some(RecordedUndetermined))
            };
            assert_eq!(
                voting_key_check_outcome(RecordedUndetermined, &VOTING_ADDRESS, slot),
                expected,
                "{slot:?}"
            );
        }
    }

    #[test]
    fn should_offer_every_key_but_a_refused_one_and_check_only_unproven_ones() {
        let use_of = |recorded, stored: Option<&[u8; 20]>| {
            let RecordedVotingKeyUse {
                can_vote,
                check_before_use,
            } = recorded_voting_key_use(recorded, stored);
            (can_vote, check_before_use)
        };
        for stored in [None, Some(&VOTING_ADDRESS)] {
            assert_eq!(use_of(None, stored), (true, true));
            assert_eq!(
                use_of(record(RecordedUndetermined, VOTING_ADDRESS), stored),
                (true, true)
            );
            assert_eq!(
                use_of(record(RecordedValid, VOTING_ADDRESS), stored),
                (true, false)
            );
            assert_eq!(
                use_of(record(RecordedInvalid, VOTING_ADDRESS), stored),
                (false, false)
            );
        }
    }

    /// A record left by another key says nothing about the stored key once its address is
    /// known: the key is offered and checked before use.
    #[test]
    fn should_treat_a_record_of_another_key_as_never_checked() {
        for check in [RecordedValid, RecordedInvalid, RecordedUndetermined] {
            assert_eq!(
                recorded_voting_key_use(record(check, OTHER_ADDRESS), Some(&VOTING_ADDRESS)),
                RecordedVotingKeyUse {
                    can_vote: true,
                    check_before_use: true,
                },
                "{check:?}"
            );
        }
    }

    #[cfg(feature = "mocks")]
    mod with_a_mock_sdk {
        use super::*;

        async fn check_against(fetched: Option<Option<Identity>>) -> VotingKeyCheck {
            let mut sdk = Sdk::new_mock();
            if let Some(fetched) = fetched {
                sdk.mock()
                    .expect_fetch::<Identity, _>(
                        voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
                        fetched,
                    )
                    .await
                    .expect("set mock expectation");
            }
            check_masternode_voting_key(&sdk, pro_tx_hash(), &VOTING_ADDRESS).await
        }

        #[tokio::test]
        async fn should_check_a_valid_pair_as_valid() {
            let identity = voter_identity(pro_tx_hash(), VOTING_ADDRESS, None);
            match check_against(Some(Some(identity))).await {
                VotingKeyCheck::Valid { voter_identity } => assert_eq!(
                    voter_identity,
                    voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS)
                ),
                other => panic!("expected Valid, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn should_check_a_pair_platform_proves_absent_as_invalid() {
            assert!(matches!(
                check_against(Some(None)).await,
                VotingKeyCheck::Invalid(VotingKeyProblem::NoVoterIdentity { .. })
            ));
        }

        /// A failed fetch says nothing about the key: it is undetermined, never invalid.
        #[tokio::test]
        async fn should_check_a_pair_whose_fetch_fails_as_undetermined() {
            assert!(matches!(
                check_against(None).await,
                VotingKeyCheck::Undetermined(_)
            ));
        }

        /// Loads the masternode against a mock that proves `owner` at the pro_tx_hash and
        /// `voter` at the voter identity of `VOTING_ADDRESS`.
        async fn load_against(
            owner: Option<Identity>,
            voter: Option<Identity>,
        ) -> Result<MasternodeVotingLoad, Error> {
            let mut sdk = Sdk::new_mock();
            sdk.mock()
                .expect_fetch::<Identity, _>(Identifier::new(pro_tx_hash().to_byte_array()), owner)
                .await
                .expect("set mock expectation");
            sdk.mock()
                .expect_fetch::<Identity, _>(
                    voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
                    voter,
                )
                .await
                .expect("set mock expectation");
            load_masternode_for_voting(&sdk, pro_tx_hash(), Some(&VOTING_ADDRESS)).await
        }

        #[tokio::test]
        async fn should_load_a_shared_masternode_through_its_voter_identity() {
            let voter = voter_identity(pro_tx_hash(), VOTING_ADDRESS, None);
            let (identity, voting_key) = loaded(load_against(None, Some(voter)).await);
            assert!(matches!(identity, MasternodeVotingIdentity::Voter(_)));
            assert_eq!(
                identity.identity().id(),
                voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS)
            );
            assert!(matches!(voting_key, Some(VotingKeyCheck::Valid { .. })));
        }

        #[tokio::test]
        async fn should_refuse_a_masternode_platform_proves_has_neither_identity() {
            assert!(matches!(
                refused(load_against(None, None).await),
                MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
                    problem: VotingKeyProblem::NoVoterIdentity { .. },
                    ..
                }
            ));
        }

        #[tokio::test]
        async fn should_refuse_a_shared_masternode_whose_voting_key_is_disabled() {
            let voter = voter_identity(pro_tx_hash(), VOTING_ADDRESS, Some(1_700_000_000));
            assert!(matches!(
                refused(load_against(None, Some(voter)).await),
                MasternodeVotingLoadRefusal::NoOwnerIdentityAndUnusableVotingKey {
                    problem: VotingKeyProblem::NoUsableVotingKey { .. },
                    ..
                }
            ));
        }

        #[tokio::test]
        async fn should_load_a_masternode_with_an_owner_identity_as_before() {
            let voter = voter_identity(pro_tx_hash(), VOTING_ADDRESS, None);
            let (identity, voting_key) =
                loaded(load_against(Some(owner_identity()), Some(voter)).await);
            assert!(matches!(identity, MasternodeVotingIdentity::Owner(_)));
            assert_eq!(identity.identity().id(), owner_identity().id());
            assert!(matches!(voting_key, Some(VotingKeyCheck::Valid { .. })));
        }

        /// The owner identity cannot be fetched (the mock holds no answer for it): nothing is
        /// proved, so the load fails rather than refusing.
        #[tokio::test]
        async fn should_fail_when_the_owner_identity_cannot_be_fetched() {
            let sdk = Sdk::new_mock();
            assert!(
                load_masternode_for_voting(&sdk, pro_tx_hash(), Some(&VOTING_ADDRESS))
                    .await
                    .is_err()
            );
        }
    }

    #[test]
    fn key_failures_are_diagnosable() {
        for cause in [
            ConsensusError::SignatureError(SignatureError::MissingPublicKeyError(
                MissingPublicKeyError::new(0),
            )),
            ConsensusError::SignatureError(SignatureError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(0),
            )),
            ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(
                IdentityNotFoundError::new(identity_id()),
            )),
        ] {
            assert!(is_voting_key_failure(&broadcast_error_with(cause.clone())));
        }
    }

    /// The discrimination the gate exists for: a signature failure that is NOT
    /// about the key's existence or state keeps its own error, as does a
    /// broadcast error carrying no consensus cause.
    #[test]
    fn unrelated_failures_are_not_diagnosable() {
        let other_signature_failure = broadcast_error_with(ConsensusError::SignatureError(
            SignatureError::BasicECDSAError(BasicECDSAError::new("bad signature".to_string())),
        ));
        assert!(!is_voting_key_failure(&other_signature_failure));

        let transport =
            Error::StateTransitionBroadcastError(crate::error::StateTransitionBroadcastError {
                code: 2,
                message: "connection reset".to_string(),
                cause: None,
            });
        assert!(!is_voting_key_failure(&transport));
    }
}
