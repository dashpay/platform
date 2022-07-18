const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createVotingIdentifier = require('./createVotingIdentifier');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @return {handleUpdatedVotingAddress}
 */
function handleUpdatedVotingAddressFactory(
  transactionalDpp,
  transactionalStateRepository,
  createMasternodeIdentity,
) {
  /**
   * @typedef handleUpdatedVotingAddress
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @return Promise<Array<Identity|Document>>
   */
  async function handleUpdatedVotingAddress(
    masternodeEntry,
  ) {
    const result = [];

    // Create a voting identity if there is no identity exist with the same ID
    const votingIdentifier = createVotingIdentifier(masternodeEntry);

    const votingIdentity = await transactionalStateRepository.fetchIdentity(votingIdentifier);

    //  Create an identity for operator if there is no identity exist with the same ID
    if (votingIdentity === null) {
      const votingAddress = Address.fromString(masternodeEntry.votingAddress);
      const votingPublicKey = Buffer.from(
        votingAddress.hashBuffer,
        'hex',
      );

      result.push(
        await createMasternodeIdentity(
          votingIdentifier,
          votingPublicKey,
          IdentityPublicKey.TYPES.ECDSA_HASH160,
        ),
      );
    }

    return result;
  }

  return handleUpdatedVotingAddress;
}

module.exports = handleUpdatedVotingAddressFactory;
