const { ChainLock } = require('@dashevo/dashcore-lib');

const ZMQClient = require('@dashevo/dashd-zmq');

const ensureBlock = require('./ensureBlock');

/**
 * Wait and ensure that core chain lock stays synced (factory)
 *
 * @param {ZMQClient} coreZMQClient
 * @param {RpcClient} coreRpcClient
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @param {BaseLogger} logger
 *
 * @returns {waitForCoreSync}
 */
function waitForCoreChainLockSyncFactory(
  coreZMQClient,
  coreRpcClient,
  latestCoreChainLock,
  logger,
) {
  /**
   * Wait and ensure that core chain lock stays synced.
   * On new ChainLock received, will also ensure that its block has been processed.
   *
   * @typedef waitForCoreChainLockSync
   *
   * @returns {Promise<void>}
   */
  async function waitForCoreChainLockSync() {
    coreZMQClient.subscribe(ZMQClient.TOPICS.rawchainlock);

    logger.trace('Subscribe to rawchainlock ZMQ room');

    let resolveFirstChainLockFromZMQPromise;
    const firstChainLockFromZMQPromise = new Promise((resolve) => {
      resolveFirstChainLockFromZMQPromise = resolve;
    });

    coreZMQClient.on(ZMQClient.TOPICS.rawchainlock, async (rawChainLockMessage) => {
      const socketChainLock = new ChainLock(rawChainLockMessage);

      await ensureBlock(coreZMQClient, coreRpcClient, socketChainLock.blockHash);

      latestCoreChainLock.update(socketChainLock);

      logger.debug(socketChainLock.toJSON(), 'Updated latestCoreChanLock');

      if (resolveFirstChainLockFromZMQPromise) {
        resolveFirstChainLockFromZMQPromise();
        resolveFirstChainLockFromZMQPromise = null;
      }
    });

    // Because a ChainLock may happen before its block, we also subscribe to rawblock
    coreZMQClient.subscribe(ZMQClient.TOPICS.hashblock);

    logger.trace('Subscribe to hashblock ZMQ topic');

    // We need to retrieve latest ChainLock from our fully synced Core instance
    let rpcBestChainLockResponse;
    try {
      rpcBestChainLockResponse = await coreRpcClient.getBestChainLock();
    } catch (e) {
      // Unable to find any ChainLock
      if (e.code === -32603) {
        logger.debug('There is no ChainLocks currently. Waiting for a first one...');

        // We need to wait for a new ChainLock from ZMQ socket
        await firstChainLockFromZMQPromise;
      } else {
        throw e;
      }
    }

    if (rpcBestChainLockResponse) {
      const chainLock = new ChainLock(rpcBestChainLockResponse.result);

      await ensureBlock(coreZMQClient, coreRpcClient, chainLock.blockHash);

      latestCoreChainLock.update(chainLock);
    }
  }

  return waitForCoreChainLockSync;
}

module.exports = waitForCoreChainLockSyncFactory;
