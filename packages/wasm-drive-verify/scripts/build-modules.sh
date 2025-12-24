#!/usr/bin/env bash
set -euo pipefail

# Always run from the package root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "Building wasm-drive-verify with ES modules..."

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf dist pkg

# Create directories
mkdir -p dist

# Build the full WASM module first
echo "Building WASM module..."
./build.sh

# Create the main index.js that re-exports everything
echo "Creating ES module wrappers..."

# Create the main index module
cat > dist/index.js << 'EOF'
// Main entry point that re-exports all modules
export * from './identity.js';
export * from './document.js';
export * from './contract.js';
export * from './tokens.js';
export * from './governance.js';
export * from './transitions.js';
export * from './core.js';
EOF

# Create core module (serialization utilities)
cat > dist/core.js << 'EOF'
import init, { 
  serialize_to_bytes,
  deserialize_from_bytes
} from '../pkg/wasm_drive_verify.js';

let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export async function serializeToBytes(value) {
  await ensureInitialized();
  return serialize_to_bytes(value);
}

export async function deserializeFromBytes(bytes) {
  await ensureInitialized();
  return deserialize_from_bytes(bytes);
}
EOF

# Create identity module
cat > dist/identity.js << 'EOF'
import init, {
  verify_full_identity_by_identity_id,
  verify_full_identity_by_unique_public_key_hash,
  verify_full_identity_by_non_unique_public_key_hash,
  verify_full_identities_by_public_key_hashes,
  verify_identity_balance_for_identity_id,
  verify_identity_balances_for_identity_ids,
  verify_identity_balance_and_revision_for_identity_id,
  verify_identity_revision_for_identity_id,
  verify_identity_nonce,
  verify_identity_contract_nonce,
  verify_identity_keys_by_identity_id,
  verify_identities_contract_keys,
  verify_identity_id_by_unique_public_key_hash,
  verify_identity_id_by_non_unique_public_key_hash,
  verify_identity_ids_by_unique_public_key_hashes
} from '../pkg/wasm_drive_verify.js';

let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export async function verifyFullIdentityByIdentityId(proof, identityId, platformVersion) {
  await ensureInitialized();
  return verify_full_identity_by_identity_id(proof, identityId, platformVersion);
}

export async function verifyFullIdentityByUniquePublicKeyHash(proof, publicKeyHash, platformVersion) {
  await ensureInitialized();
  return verify_full_identity_by_unique_public_key_hash(proof, publicKeyHash, platformVersion);
}

export async function verifyFullIdentityByNonUniquePublicKeyHash(proof, publicKeyHash, platformVersion) {
  await ensureInitialized();
  return verify_full_identity_by_non_unique_public_key_hash(proof, publicKeyHash, platformVersion);
}

export async function verifyFullIdentitiesByPublicKeyHashes(proof, publicKeyHashes, platformVersion) {
  await ensureInitialized();
  return verify_full_identities_by_public_key_hashes(proof, publicKeyHashes, platformVersion);
}

export async function verifyIdentityBalanceForIdentityId(proof, identityId, platformVersion) {
  await ensureInitialized();
  return verify_identity_balance_for_identity_id(proof, identityId, platformVersion);
}

export async function verifyIdentityBalancesForIdentityIds(proof, identityIds, platformVersion) {
  await ensureInitialized();
  return verify_identity_balances_for_identity_ids(proof, identityIds, platformVersion);
}

export async function verifyIdentityBalanceAndRevisionForIdentityId(proof, identityId, platformVersion) {
  await ensureInitialized();
  return verify_identity_balance_and_revision_for_identity_id(proof, identityId, platformVersion);
}

export async function verifyIdentityRevisionForIdentityId(proof, identityId, platformVersion) {
  await ensureInitialized();
  return verify_identity_revision_for_identity_id(proof, identityId, platformVersion);
}

export async function verifyIdentityNonce(proof, identityId, contractId, platformVersion) {
  await ensureInitialized();
  return verify_identity_nonce(proof, identityId, contractId, platformVersion);
}

export async function verifyIdentityContractNonce(proof, identityId, contractId, documentTypeName, platformVersion) {
  await ensureInitialized();
  return verify_identity_contract_nonce(proof, identityId, contractId, documentTypeName, platformVersion);
}

export async function verifyIdentityKeysByIdentityId(proof, identityId, keyRequestType, limit, offset, platformVersion) {
  await ensureInitialized();
  return verify_identity_keys_by_identity_id(proof, identityId, keyRequestType, limit, offset, platformVersion);
}

export async function verifyIdentitiesContractKeys(proof, identityIds, contractId, documentTypeName, purposes, platformVersion) {
  await ensureInitialized();
  return verify_identities_contract_keys(proof, identityIds, contractId, documentTypeName, purposes, platformVersion);
}

export async function verifyIdentityIdByUniquePublicKeyHash(proof, publicKeyHash, platformVersion) {
  await ensureInitialized();
  return verify_identity_id_by_unique_public_key_hash(proof, publicKeyHash, platformVersion);
}

export async function verifyIdentityIdByNonUniquePublicKeyHash(proof, publicKeyHash, platformVersion) {
  await ensureInitialized();
  return verify_identity_id_by_non_unique_public_key_hash(proof, publicKeyHash, platformVersion);
}

export async function verifyIdentityIdsByUniquePublicKeyHashes(proof, publicKeyHashes, platformVersion) {
  await ensureInitialized();
  return verify_identity_ids_by_unique_public_key_hashes(proof, publicKeyHashes, platformVersion);
}
EOF

# Create document module
cat > dist/document.js << 'EOF'
import init, {
  verify_proof,
  verify_proof_keep_serialized,
  verify_start_at_document_in_proof,
  verify_single_document
} from '../pkg/wasm_drive_verify.js';

let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export async function verifyProof(proof, contractId, documentTypeName, query, platformVersion) {
  await ensureInitialized();
  return verify_proof(proof, contractId, documentTypeName, query, platformVersion);
}

export async function verifyProofKeepSerialized(proof, contractId, documentTypeName, query, platformVersion) {
  await ensureInitialized();
  return verify_proof_keep_serialized(proof, contractId, documentTypeName, query, platformVersion);
}

export async function verifyStartAtDocumentInProof(proof, contractId, documentTypeName, query, platformVersion) {
  await ensureInitialized();
  return verify_start_at_document_in_proof(proof, contractId, documentTypeName, query, platformVersion);
}

export async function verifySingleDocument(proof, documentId, contractId, documentType, platformVersion) {
  await ensureInitialized();
  return verify_single_document(proof, documentId, contractId, documentType, platformVersion);
}
EOF

# Create contract module
cat > dist/contract.js << 'EOF'
import init, {
  verify_contract,
  verify_contract_history
} from '../pkg/wasm_drive_verify.js';

let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export async function verifyContract(proof, contractId, platformVersion) {
  await ensureInitialized();
  return verify_contract(proof, contractId, platformVersion);
}

export async function verifyContractHistory(proof, contractId, limit, offset, startAtMs, platformVersion) {
  await ensureInitialized();
  return verify_contract_history(proof, contractId, limit, offset, startAtMs, platformVersion);
}
EOF

# Create tokens module
cat > dist/tokens.js << 'EOF'
import init, {
  verify_token_balance_for_identity_id,
  verify_token_balances_for_identity_id,
  verify_token_balances_for_identity_ids,
  verify_token_info_for_identity_id,
  verify_token_infos_for_identity_id,
  verify_token_infos_for_identity_ids,
  verify_token_contract_info,
  verify_token_status,
  verify_token_statuses,
  verify_token_direct_selling_price,
  verify_token_direct_selling_prices,
  verify_token_pre_programmed_distributions,
  verify_token_perpetual_distribution_last_paid_time,
  verify_token_total_supply_and_aggregated_identity_balance
} from '../pkg/wasm_drive_verify.js';

let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export async function verifyTokenBalanceForIdentityId(proof, contractId, identityId, platformVersion) {
  await ensureInitialized();
  return verify_token_balance_for_identity_id(proof, contractId, identityId, platformVersion);
}

export async function verifyTokenBalancesForIdentityId(proof, identityId, platformVersion) {
  await ensureInitialized();
  return verify_token_balances_for_identity_id(proof, identityId, platformVersion);
}

export async function verifyTokenBalancesForIdentityIds(proof, contractId, identityIds, platformVersion) {
  await ensureInitialized();
  return verify_token_balances_for_identity_ids(proof, contractId, identityIds, platformVersion);
}

export async function verifyTokenInfoForIdentityId(proof, contractId, identityId, platformVersion) {
  await ensureInitialized();
  return verify_token_info_for_identity_id(proof, contractId, identityId, platformVersion);
}

export async function verifyTokenInfosForIdentityId(proof, identityId, platformVersion) {
  await ensureInitialized();
  return verify_token_infos_for_identity_id(proof, identityId, platformVersion);
}

export async function verifyTokenInfosForIdentityIds(proof, contractId, identityIds, platformVersion) {
  await ensureInitialized();
  return verify_token_infos_for_identity_ids(proof, contractId, identityIds, platformVersion);
}

export async function verifyTokenContractInfo(proof, contractId, platformVersion) {
  await ensureInitialized();
  return verify_token_contract_info(proof, contractId, platformVersion);
}

export async function verifyTokenStatus(proof, contractId, platformVersion) {
  await ensureInitialized();
  return verify_token_status(proof, contractId, platformVersion);
}

export async function verifyTokenStatuses(proof, contractIds, platformVersion) {
  await ensureInitialized();
  return verify_token_statuses(proof, contractIds, platformVersion);
}

export async function verifyTokenDirectSellingPrice(proof, contractId, platformVersion) {
  await ensureInitialized();
  return verify_token_direct_selling_price(proof, contractId, platformVersion);
}

export async function verifyTokenDirectSellingPrices(proof, contractIds, platformVersion) {
  await ensureInitialized();
  return verify_token_direct_selling_prices(proof, contractIds, platformVersion);
}

export async function verifyTokenPreProgrammedDistributions(proof, contractId, platformVersion) {
  await ensureInitialized();
  return verify_token_pre_programmed_distributions(proof, contractId, platformVersion);
}

export async function verifyTokenPerpetualDistributionLastPaidTime(proof, contractId, platformVersion) {
  await ensureInitialized();
  return verify_token_perpetual_distribution_last_paid_time(proof, contractId, platformVersion);
}

export async function verifyTokenTotalSupplyAndAggregatedIdentityBalance(proof, contractId, platformVersion) {
  await ensureInitialized();
  return verify_token_total_supply_and_aggregated_identity_balance(proof, contractId, platformVersion);
}
EOF

# Create governance module
cat > dist/governance.js << 'EOF'
import init, {
  // Group functions
  verify_group_info,
  verify_group_infos_in_contract,
  verify_action_signers,
  verify_action_signers_total_power,
  verify_active_action_infos,
  // Voting functions
  verify_vote_poll_vote_state_proof,
  verify_vote_poll_votes_proof,
  verify_vote_polls_end_date_query,
  verify_contests_proof,
  verify_identity_votes_given_proof,
  verify_masternode_vote,
  verify_specialized_balance,
  // System functions
  verify_total_credits_in_system,
  verify_upgrade_state,
  verify_upgrade_vote_status,
  verify_epoch_infos,
  verify_epoch_proposers,
  verify_elements
} from '../pkg/wasm_drive_verify.js';

let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

// Group exports
export async function verifyGroupInfo(proof, contractId, groupId, platformVersion) {
  await ensureInitialized();
  return verify_group_info(proof, contractId, groupId, platformVersion);
}

export async function verifyGroupInfosInContract(proof, contractId, limit, offset, platformVersion) {
  await ensureInitialized();
  return verify_group_infos_in_contract(proof, contractId, limit, offset, platformVersion);
}

export async function verifyActionSigners(proof, contractId, groupId, actionId, platformVersion) {
  await ensureInitialized();
  return verify_action_signers(proof, contractId, groupId, actionId, platformVersion);
}

export async function verifyActionSignersTotalPower(proof, contractId, groupId, actionId, platformVersion) {
  await ensureInitialized();
  return verify_action_signers_total_power(proof, contractId, groupId, actionId, platformVersion);
}

export async function verifyActiveActionInfos(proof, identityId, limit, offset, platformVersion) {
  await ensureInitialized();
  return verify_active_action_infos(proof, identityId, limit, offset, platformVersion);
}

// Voting exports
export async function verifyVotePollVoteStateProof(proof, votePollId, platformVersion) {
  await ensureInitialized();
  return verify_vote_poll_vote_state_proof(proof, votePollId, platformVersion);
}

export async function verifyVotePollVotesProof(proof, votePollId, contestantId, startIdentityIdIncluded, endIdentityIdExcluded, allowInclusion, platformVersion) {
  await ensureInitialized();
  return verify_vote_poll_votes_proof(proof, votePollId, contestantId, startIdentityIdIncluded, endIdentityIdExcluded, allowInclusion, platformVersion);
}

export async function verifyVotePollsEndDateQuery(proof, limit, offset, ascending, platformVersion) {
  await ensureInitialized();
  return verify_vote_polls_end_date_query(proof, limit, offset, ascending, platformVersion);
}

export async function verifyContestsProof(proof, limit, offset, orderAscending, includeLockedVotes, memberIdentityId, platformVersion) {
  await ensureInitialized();
  return verify_contests_proof(proof, limit, offset, orderAscending, includeLockedVotes, memberIdentityId, platformVersion);
}

export async function verifyIdentityVotesGivenProof(proof, identityId, limit, offset, orderAscending, platformVersion) {
  await ensureInitialized();
  return verify_identity_votes_given_proof(proof, identityId, limit, offset, orderAscending, platformVersion);
}

export async function verifyMasternodeVote(proof, proTxHash, resourceVoteChoice, platformVersion) {
  await ensureInitialized();
  return verify_masternode_vote(proof, proTxHash, resourceVoteChoice, platformVersion);
}

export async function verifySpecializedBalance(proof, balanceId, platformVersion) {
  await ensureInitialized();
  return verify_specialized_balance(proof, balanceId, platformVersion);
}

// System exports
export async function verifyTotalCreditsInSystem(proof, platformVersion) {
  await ensureInitialized();
  return verify_total_credits_in_system(proof, platformVersion);
}

export async function verifyUpgradeState(proof, platformVersion) {
  await ensureInitialized();
  return verify_upgrade_state(proof, platformVersion);
}

export async function verifyUpgradeVoteStatus(proof, startProTxHash, count, platformVersion) {
  await ensureInitialized();
  return verify_upgrade_vote_status(proof, startProTxHash, count, platformVersion);
}

export async function verifyEpochInfos(proof, startEpoch, count, ascending, platformVersion) {
  await ensureInitialized();
  return verify_epoch_infos(proof, startEpoch, count, ascending, platformVersion);
}

export async function verifyEpochProposers(proof, epoch, limit, offset, platformVersion) {
  await ensureInitialized();
  return verify_epoch_proposers(proof, epoch, limit, offset, platformVersion);
}

export async function verifyElements(proof, path, keys, platformVersion) {
  await ensureInitialized();
  return verify_elements(proof, path, keys, platformVersion);
}
EOF

# Create transitions module
cat > dist/transitions.js << 'EOF'
import init, {
  verify_state_transition_was_executed_with_proof
} from '../pkg/wasm_drive_verify.js';

let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export async function verifyStateTransitionWasExecutedWithProof(proof, stateTransitionHash, platformVersion) {
  await ensureInitialized();
  return verify_state_transition_was_executed_with_proof(proof, stateTransitionHash, platformVersion);
}
EOF

# Create TypeScript declaration files
echo "Creating TypeScript declaration files..."

# Create index.d.ts
cat > dist/index.d.ts << 'EOF'
export * from './identity';
export * from './document';
export * from './contract';
export * from './tokens';
export * from './governance';
export * from './transitions';
export * from './core';
EOF

# Create core.d.ts
cat > dist/core.d.ts << 'EOF'
export function serializeToBytes(value: any): Promise<Uint8Array>;
export function deserializeFromBytes(bytes: Uint8Array): Promise<any>;
EOF

# Copy other .d.ts files (we'll generate these properly later)
echo "TypeScript declarations will be generated from Rust types..."

echo "ES module build complete!"
echo "Modules are available in the dist/ directory"