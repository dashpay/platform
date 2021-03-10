const wait = require('../util/wait');

/**
 * Wait for confirmations to be reached
 * @typedef waitForConfirmations
 * @param {CoreService} coreService
 * @param {string} txHash
 * @param {number} confirmations
 * @param {function(confirmations: number)} [progressCallback]
 * @returns {Promise<void>}
 */
async function waitForConfirmations(
  coreService,
  txHash,
  confirmations,
  progressCallback = () => {},
) {
  let confirmationsReached = 0;

  do {
    await wait(20000);
    ({ result: { confirmations: confirmationsReached } } = await coreService
      .getRpcClient()
      .getrawtransaction(txHash, 1));

    if (confirmationsReached === undefined) {
      confirmationsReached = 0;
    }

    await progressCallback(confirmationsReached);
  } while (confirmationsReached < confirmations);
}

module.exports = waitForConfirmations;
