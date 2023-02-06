const { convertSatoshiToCredits } = require('../../creditsConverter');

/**
 * @param {StateRepository} stateRepository
 * @param {fetchAssetLockTransactionOutput} fetchAssetLockTransactionOutput
 *
 * @returns {applyIdentityTopUpTransition}
 */
function applyIdentityTopUpTransitionFactory(
  stateRepository,
  fetchAssetLockTransactionOutput,
) {
  /**
   * Apply identity state transition
   *
   * @typedef applyIdentityTopUpTransition
   *
   * @param {IdentityTopUpTransition} stateTransition
   *
   * @return {Promise<void>}
   */
  async function applyIdentityTopUpTransition(stateTransition) {
    const executionContext = stateTransition.getExecutionContext();

    const output = await fetchAssetLockTransactionOutput(
      stateTransition.getAssetLockProof(),
      executionContext,
    );

    const outPoint = stateTransition.getAssetLockProof().getOutPoint();

    let creditsAmount = convertSatoshiToCredits(output.satoshis);

    const identityId = stateTransition.getIdentityId();

    await stateRepository.addToIdentityBalance(identityId, creditsAmount, executionContext);

    // Ignore balance dept for system credits
    const balance = await stateRepository.fetchIdentityBalanceWithDebt(
      identityId,
      executionContext,
    );

    if (balance < 0) {
      creditsAmount += balance;
    }

    await stateRepository.addToSystemCredits(creditsAmount, executionContext);

    await stateRepository.markAssetLockTransactionOutPointAsUsed(outPoint, executionContext);
  }

  return applyIdentityTopUpTransition;
}

module.exports = applyIdentityTopUpTransitionFactory;
