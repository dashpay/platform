const MissingChainLockError = require('./errors/MissingChainLockError');
const LatestCoreChainLock = require('./LatestCoreChainLock');

/**
 *
 * @param {LatestCoreChainLock} latestCoreChainLock

 * @return {waitForChainLockedHeight}
 */
function waitForChainLockedHeightFactory(
  latestCoreChainLock,
) {
  /**
   * @typedef waitForChainLockedHeight
   * @param {number} coreHeight
   *
   * @return {Promise<void>}
   */
  async function waitForChainLockedHeight(coreHeight) {
    // ChainLock is required to get finalized SML that won't be reorged
    const existingChainLock = latestCoreChainLock.getChainLock();

    if (!existingChainLock) {
      throw new MissingChainLockError();
    }

    // Wait for core to be synced up to coreHeight
    if (coreHeight > existingChainLock.height) {
      await new Promise((resolve) => {
        const listener = (chainLock) => {
          // Skip if core height still not reached
          if (coreHeight > chainLock.height) {
            return;
          }

          latestCoreChainLock.removeListener(LatestCoreChainLock.EVENTS.update, listener);

          resolve();
        };

        latestCoreChainLock.on(LatestCoreChainLock.EVENTS.update, listener);
      });
    }
  }

  return waitForChainLockedHeight;
}

module.exports = waitForChainLockedHeightFactory;
