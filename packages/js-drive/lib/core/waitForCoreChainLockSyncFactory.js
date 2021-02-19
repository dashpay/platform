const { ChainLock } = require('@dashevo/dashcore-lib');

const ChainLockSigMessage = require('@dashevo/dashcore-lib/lib/zmqMessages/ChainLockSigMessage');
const ZMQClient = require('./ZmqClient');

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
    coreZMQClient.subscribe(ZMQClient.TOPICS.rawchainlocksig);

    let resolveFirstChainLockFromZMQPromise;
    const firstChainLockFromZMQPromise = new Promise((resolve) => {
      resolveFirstChainLockFromZMQPromise = resolve;
    });

    coreZMQClient.on(ZMQClient.TOPICS.rawchainlocksig, async (rawChainLockMessage) => {
      let chainLock;

      try {
        ({ chainLock } = new ChainLockSigMessage(rawChainLockMessage));
      } catch (e) {
        logger.error({ err: e }, 'Error on creating ChainLockSigMessage');
        logger.debug({
          rawChainLockMessage: rawChainLockMessage.toString('hex'),
        });

        return;
      }

      latestCoreChainLock.update(chainLock);

      logger.trace(`Updated latestCoreChanLock for core height ${chainLock.height}`);

      if (resolveFirstChainLockFromZMQPromise) {
        resolveFirstChainLockFromZMQPromise();
        resolveFirstChainLockFromZMQPromise = null;
      }
    });

    // Because a ChainLock may happen before its block, we also subscribe to rawblock
    coreZMQClient.subscribe(ZMQClient.TOPICS.hashblock);

    // We need to retrieve latest ChainLock from our fully synced Core instance
    let rpcBestChainLockResponse;
    try {
      rpcBestChainLockResponse = await coreRpcClient.getBestChainLock();
    } catch (e) {
      // Unable to find any ChainLock
      if (e.code === -32603) {
        logger.debug('There is no chain locks currently. Waiting for a first one...');

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
