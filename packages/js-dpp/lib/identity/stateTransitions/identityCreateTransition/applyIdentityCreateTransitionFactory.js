const Identity = require('../../Identity');

const { convertSatoshiToCredits } = require('../../creditsConverter');

/**
 * @param {StateRepository} stateRepository
 * @param {fetchConfirmedAssetLockTransactionOutput} fetchConfirmedAssetLockTransactionOutput
 *
 * @returns {applyIdentityCreateTransition}
 */
function applyIdentityCreateTransitionFactory(
  stateRepository,
  fetchConfirmedAssetLockTransactionOutput,
) {
  /**
   * Apply identity state transition
   *
   * @typedef applyIdentityCreateTransition
   *
   * @param {IdentityCreateTransition} stateTransition
   *
   * @return {Promise<void>}
   */
  async function applyIdentityCreateTransition(stateTransition) {
    const output = await fetchConfirmedAssetLockTransactionOutput(
      stateTransition.getLockedOutPoint(),
    );

    const creditsAmount = convertSatoshiToCredits(output.satoshis);

    const identity = new Identity({
      protocolVersion: stateTransition.getProtocolVersion(),
      id: stateTransition.getIdentityId(),
      publicKeys: stateTransition.getPublicKeys().map((key) => key.toJSON()),
      balance: creditsAmount,
      revision: 0,
    });

    await stateRepository.storeIdentity(identity);

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    await stateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      publicKeyHashes,
    );
  }

  return applyIdentityCreateTransition;
}

module.exports = applyIdentityCreateTransitionFactory;
