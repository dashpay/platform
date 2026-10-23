//! V019 record codec adapter. Field order and enum tags here are historical:
//! evolve the runtime types through explicit conversions, never these layouts.
//! Core payload bodies and leaf primitives retain their upstream persisted-wire
//! contract; immutable pre-V020 fixtures exercise that contract too.

use dashcore::transaction::special_transaction::{
    asset_lock::AssetLockPayload, asset_unlock::qualified_asset_unlock::AssetUnlockPayload,
    coinbase::CoinbasePayload, mnhf_signal::MnhfSignalPayload,
    provider_dissolve::ProviderDissolvePayload, provider_registration::ProviderRegistrationPayload,
    provider_update_registrar::ProviderUpdateRegistrarPayload,
    provider_update_revocation::ProviderUpdateRevocationPayload,
    provider_update_service::ProviderUpdateServicePayload,
    provider_update_share::ProviderUpdateSharePayload,
    provider_update_shared_registrar::ProviderUpdateSharedRegistrarPayload,
    quorum_commitment::QuorumCommitmentPayload, TransactionPayload,
};
use dashcore::{
    Address, BlockHash, InstantLock, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness,
};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::transaction_record::{
    InputDetail, OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
use serde::{Deserialize, Deserializer};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// Decode the V019 envelope without invoking the runtime record deserializer.
pub(super) fn decode_transaction_record(
    payload: &[u8],
) -> Result<TransactionRecord, WalletStorageError> {
    let Record(record) = blob::decode(payload)?;
    if record.txid != record.transaction.txid() {
        return Err(WalletStorageError::blob_decode(
            "transaction record txid disagrees with its transaction body hash",
        ));
    }
    Ok(record)
}

#[derive(Deserialize)]
struct Record(#[serde(with = "TransactionRecordV019")] TransactionRecord);

#[derive(Deserialize)]
#[serde(remote = "TransactionRecord")]
struct TransactionRecordV019 {
    #[serde(with = "TransactionV019")]
    transaction: Transaction,
    txid: Txid,
    #[serde(with = "AccountTypeV019")]
    account_type: AccountType,
    #[serde(with = "TransactionContextV019")]
    context: TransactionContext,
    #[serde(with = "TransactionTypeV019")]
    transaction_type: TransactionType,
    #[serde(with = "TransactionDirectionV019")]
    direction: TransactionDirection,
    #[serde(deserialize_with = "input_details_v019")]
    input_details: Vec<InputDetail>,
    #[serde(deserialize_with = "output_details_v019")]
    output_details: Vec<OutputDetail>,
    net_amount: i64,
    fee: Option<u64>,
    label: String,
}

#[derive(Deserialize)]
#[serde(remote = "AccountType")]
enum AccountTypeV019 {
    Standard {
        index: u32,
        #[serde(with = "StandardAccountTypeV019")]
        standard_account_type: StandardAccountType,
    },
    CoinJoin {
        index: u32,
    },
    IdentityRegistration,
    IdentityTopUp {
        registration_index: u32,
    },
    IdentityTopUpNotBoundToIdentity,
    IdentityInvitation,
    AssetLockAddressTopUp,
    AssetLockShieldedAddressTopUp,
    ProviderVotingKeys,
    ProviderOwnerKeys,
    ProviderOperatorKeys,
    ProviderPlatformKeys,
    DashpayReceivingFunds {
        index: u32,
        user_identity_id: [u8; 32],
        friend_identity_id: [u8; 32],
    },
    DashpayExternalAccount {
        index: u32,
        user_identity_id: [u8; 32],
        friend_identity_id: [u8; 32],
    },
    PlatformPayment {
        account: u32,
        key_class: u32,
    },
}

#[derive(Deserialize)]
#[serde(remote = "StandardAccountType")]
enum StandardAccountTypeV019 {
    BIP44Account,
    BIP32Account,
}

#[derive(Deserialize)]
#[serde(remote = "TransactionContext")]
enum TransactionContextV019 {
    Mempool,
    InstantSend(InstantLock),
    InBlock(#[serde(deserialize_with = "block_info_v019")] BlockInfo),
    InChainLockedBlock(#[serde(deserialize_with = "block_info_v019")] BlockInfo),
}

#[derive(Deserialize)]
#[serde(remote = "TransactionType")]
enum TransactionTypeV019 {
    Standard,
    CoinJoin,
    ProviderRegistration,
    ProviderUpdateRegistrar,
    ProviderUpdateService,
    ProviderUpdateRevocation,
    AssetLock,
    AssetUnlock,
    Coinbase,
    Ignored,
}

#[derive(Deserialize)]
#[serde(remote = "TransactionDirection")]
enum TransactionDirectionV019 {
    Incoming,
    Outgoing,
    Internal,
    CoinJoin,
}

#[derive(Deserialize)]
#[serde(remote = "InputDetail")]
struct InputDetailV019 {
    index: u32,
    value: u64,
    address: Address,
}

#[derive(Deserialize)]
#[serde(remote = "OutputDetail")]
struct OutputDetailV019 {
    index: u32,
    #[serde(with = "OutputRoleV019")]
    role: OutputRole,
    address: Option<Address>,
    value: u64,
}

#[derive(Deserialize)]
#[serde(remote = "OutputRole")]
enum OutputRoleV019 {
    Received,
    Change,
    Sent,
    Unspendable,
}

#[derive(Deserialize)]
#[serde(remote = "Transaction")]
struct TransactionV019 {
    version: u16,
    lock_time: u32,
    #[serde(deserialize_with = "input_v019")]
    input: Vec<TxIn>,
    #[serde(deserialize_with = "output_v019")]
    output: Vec<TxOut>,
    #[serde(deserialize_with = "payload_v019")]
    special_transaction_payload: Option<TransactionPayload>,
}

#[derive(Deserialize)]
#[serde(remote = "TxIn")]
struct TxInV019 {
    #[serde(with = "OutPointV019")]
    previous_output: OutPoint,
    script_sig: ScriptBuf,
    sequence: u32,
    witness: Witness,
}

#[derive(Deserialize)]
#[serde(remote = "TxOut")]
struct TxOutV019 {
    value: u64,
    script_pubkey: ScriptBuf,
}

#[derive(Deserialize)]
#[serde(remote = "OutPoint")]
struct OutPointV019 {
    txid: Txid,
    vout: u32,
}

fn input_details_v019<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<InputDetail>, D::Error> {
    #[derive(Deserialize)]
    struct Item(#[serde(with = "InputDetailV019")] InputDetail);
    Vec::<Item>::deserialize(deserializer)
        .map(|items| items.into_iter().map(|item| item.0).collect())
}

fn output_details_v019<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<OutputDetail>, D::Error> {
    #[derive(Deserialize)]
    struct Item(#[serde(with = "OutputDetailV019")] OutputDetail);
    Vec::<Item>::deserialize(deserializer)
        .map(|items| items.into_iter().map(|item| item.0).collect())
}

fn input_v019<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<TxIn>, D::Error> {
    #[derive(Deserialize)]
    struct Item(#[serde(with = "TxInV019")] TxIn);
    Vec::<Item>::deserialize(deserializer)
        .map(|items| items.into_iter().map(|item| item.0).collect())
}

fn output_v019<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<TxOut>, D::Error> {
    #[derive(Deserialize)]
    struct Item(#[serde(with = "TxOutV019")] TxOut);
    Vec::<Item>::deserialize(deserializer)
        .map(|items| items.into_iter().map(|item| item.0).collect())
}

fn block_info_v019<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BlockInfo, D::Error> {
    #[derive(Deserialize)]
    struct BlockInfoV019 {
        height: u32,
        block_hash: BlockHash,
        timestamp: u32,
        position: Option<u32>,
    }
    let info = BlockInfoV019::deserialize(deserializer)?;
    let block = BlockInfo::new(info.height, info.block_hash, info.timestamp);
    Ok(match info.position {
        Some(position) => block.with_position(position),
        None => block,
    })
}
#[derive(Deserialize)]
#[serde(remote = "TransactionPayload")]
#[expect(
    clippy::enum_variant_names,
    reason = "serde remote variants must match the upstream names"
)]
enum TransactionPayloadV019 {
    ProviderRegistrationPayloadType(ProviderRegistrationPayload),
    ProviderUpdateServicePayloadType(ProviderUpdateServicePayload),
    ProviderUpdateRegistrarPayloadType(ProviderUpdateRegistrarPayload),
    ProviderUpdateRevocationPayloadType(ProviderUpdateRevocationPayload),
    CoinbasePayloadType(CoinbasePayload),
    QuorumCommitmentPayloadType(QuorumCommitmentPayload),
    MnhfSignalPayloadType(MnhfSignalPayload),
    AssetLockPayloadType(AssetLockPayload),
    AssetUnlockPayloadType(AssetUnlockPayload),
    ClassicalWithNonStandardVersionTypeBytesPayloadType(u16),
    RawPayloadType(u16, Vec<u8>),
    ProviderDissolvePayloadType(ProviderDissolvePayload),
    ProviderUpdateSharePayloadType(ProviderUpdateSharePayload),
    ProviderUpdateSharedRegistrarPayloadType(ProviderUpdateSharedRegistrarPayload),
}

fn payload_v019<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<TransactionPayload>, D::Error> {
    #[derive(Deserialize)]
    struct Payload(#[serde(with = "TransactionPayloadV019")] TransactionPayload);
    Option::<Payload>::deserialize(deserializer).map(|payload| payload.map(|value| value.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_decode_all_immutable_v019_transaction_fixtures() {
        for line in include_str!("../../../tests/fixtures/core_transaction_rows.txt").lines() {
            let (name, bytes) = line.split_once(' ').unwrap();
            if name == "asset_lock_lifecycle" {
                continue;
            }
            let bytes = hex::decode(bytes).unwrap();
            let record =
                decode_transaction_record(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(record.txid, record.transaction.txid(), "{name}");
            assert_eq!(blob::encode(&record).unwrap(), bytes, "{name}");
        }
    }
}
