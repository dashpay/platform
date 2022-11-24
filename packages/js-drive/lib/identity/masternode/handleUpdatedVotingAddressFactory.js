const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const createVotingIdentifier = require('./createVotingIdentifier');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @param {fetchTransaction} fetchTransaction
 * @return {handleUpdatedVotingAddress}
 */
function handleUpdatedVotingAddressFactory(
  identityRepository,
  createMasternodeIdentity,
  fetchTransaction,
) {
  /**
   * @typedef handleUpdatedVotingAddress
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {GroveDBTransaction} transaction
   * @return Promise<Array<Identity|Document>>
   */
  async function handleUpdatedVotingAddress(
    masternodeEntry,
    transaction,
  ) {
    const result = [];

    const { extraPayload: proRegTxPayload } = await fetchTransaction(masternodeEntry.proRegTxHash);

    const ownerPublicKeyHash = Buffer.from(proRegTxPayload.keyIDOwner, 'hex').reverse();
    const votingPubKeyHash = Buffer.from(proRegTxPayload.keyIDVoting, 'hex').reverse();

    // don't need to create a separate Identity in case we don't have
    // public key used for proposal voting (keyIDVoting === keyIDOwner)
    if (ownerPublicKeyHash.equals(votingPubKeyHash)) {
      return result;
    }

    // Create a voting identity if there is no identity exist with the same ID
    const votingIdentifier = createVotingIdentifier(masternodeEntry);

    const votingIdentityResult = await identityRepository.fetch(
      votingIdentifier,
      { transaction },
    );

    //  Create an identity for operator if there is no identity exist with the same ID
    if (votingIdentityResult.isNull()) {
      const votingAddress = Address.fromString(masternodeEntry.votingAddress);
      const votingPublicKeyHash = votingAddress.hashBuffer;

      result.push(
        await createMasternodeIdentity(
          votingIdentifier,
          votingPublicKeyHash,
          IdentityPublicKey.TYPES.ECDSA_HASH160,
          transaction,
        ),
      );
    }

    return result;
  }

  return handleUpdatedVotingAddress;
}

module.exports = handleUpdatedVotingAddressFactory;
