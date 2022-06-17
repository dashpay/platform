const Identity = require('../../Identity');

const { convertSatoshiToCredits } = require('../../creditsConverter');

/**
 * @param {StateRepository} stateRepository
 * @param {fetchAssetLockTransactionOutput} fetchAssetLockTransactionOutput
 *
 * @returns {applyIdentityCreateTransition}
 */
function applyIdentityCreateTransitionFactory(
  stateRepository,
  fetchAssetLockTransactionOutput,
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
    const executionContext = stateTransition.getExecutionContext();

    const output = await fetchAssetLockTransactionOutput(
      stateTransition.getAssetLockProof(),
      executionContext,
    );

    const creditsAmount = convertSatoshiToCredits(output.satoshis);

    const identity = new Identity({
      protocolVersion: stateTransition.getProtocolVersion(),
      id: stateTransition.getIdentityId().toBuffer(),
      publicKeys: stateTransition.getPublicKeys()
        .map((key) => key.toObject({ skipSignature: true })),
      balance: creditsAmount,
      revision: 0,
    });

    await stateRepository.createIdentity(identity, executionContext);

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    await stateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      publicKeyHashes,
      executionContext,
    );

    const outPoint = stateTransition.getAssetLockProof().getOutPoint();

    await stateRepository.markAssetLockTransactionOutPointAsUsed(outPoint, executionContext);
  }

  return applyIdentityCreateTransition;
}

module.exports = applyIdentityCreateTransitionFactory;
