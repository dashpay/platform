const {
  tendermint: {
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');
/**
 *
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @return {updateCoreChainLock}
 */
function updateCoreChainLockFactory(
  blockExecutionContext,
  latestCoreChainLock,
) {
  /**
   * @typedef updateCoreChainLock
   * @param {BaseLogger} logger
   * @return {Promise<CoreChainLock>}
   */
  async function updateCoreChainLock(logger) {
    const consensusLogger = logger.child({
      abciMethod: 'updateCoreChainLock',
    });

    // Update Core Chain Locks

    const contextCoreChainLockedHeight = blockExecutionContext.getCoreChainLockedHeight();
    const coreChainLock = latestCoreChainLock.getChainLock();

    let coreChainLockUpdate;
    if (coreChainLock && coreChainLock.height > contextCoreChainLockedHeight) {
      coreChainLockUpdate = new CoreChainLock({
        coreBlockHeight: coreChainLock.height,
        coreBlockHash: coreChainLock.blockHash,
        signature: coreChainLock.signature,
      });

      consensusLogger.trace(
        {
          nextCoreChainLockHeight: coreChainLock.height,
        },
        `Provide next chain lock for Core height ${coreChainLock.height}`,
      );
    }

    return coreChainLockUpdate;
  }

  return updateCoreChainLock;
}

module.exports = updateCoreChainLockFactory;
