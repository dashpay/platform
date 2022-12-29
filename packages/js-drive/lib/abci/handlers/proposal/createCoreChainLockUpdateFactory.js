const {
  tendermint: {
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');
/**
 *
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @return {createCoreChainLockUpdate}
 */
function createCoreChainLockUpdateFactory(
  latestCoreChainLock,
) {
  /**
   * @typedef createCoreChainLockUpdate
   * @param {number} contextCoreChainLockedHeight
   * @param {number} round
   * @param {BaseLogger} consensusLogger
   * @return {Promise<CoreChainLock>}
   */
  async function createCoreChainLockUpdate(contextCoreChainLockedHeight, round, consensusLogger) {
    // Update Core Chain Locks
    const coreChainLock = latestCoreChainLock.getChainLock();

    let coreChainLockUpdate;
    if (coreChainLock && coreChainLock.height > contextCoreChainLockedHeight) {
      coreChainLockUpdate = new CoreChainLock({
        coreBlockHeight: coreChainLock.height,
        coreBlockHash: coreChainLock.blockHash,
        signature: coreChainLock.signature,
      });

      consensusLogger.debug(
        {
          nextCoreChainLockHeight: coreChainLock.height,
        },
        `Provide next chain lock for Core height ${coreChainLock.height}`,
      );
    }

    return coreChainLockUpdate;
  }

  return createCoreChainLockUpdate;
}

module.exports = createCoreChainLockUpdateFactory;
