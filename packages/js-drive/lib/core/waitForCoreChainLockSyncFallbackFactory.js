const { ChainLock } = require('@dashevo/dashcore-lib');

const ZMQClient = require('@dashevo/dashd-zmq');

/**
 *
 * @param {ZMQClient} coreZMQClient
 * @param {RpcClient} coreRpcClient
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @param {BaseLogger} logger
 *
 * @returns {waitForCoreChainLockSyncFallback}
 */
function waitForCoreChainLockSyncFallbackFactory(
  coreZMQClient,
  coreRpcClient,
  latestCoreChainLock,
  logger,
) {
  /**
   * @typedef waitForCoreChainLockSyncFallback
   *
   * @return {Promise<void>}
   */
  async function waitForCoreChainLockSyncFallback() {
    const signature = Buffer.alloc(96).toString('hex');

    coreZMQClient.subscribe(ZMQClient.TOPICS.hashblock);

    logger.trace('Subscribe to hashblock ZMQ room');

    let resolveFirstBlockFromZMQPromise;
    const firstBlockFromZMQPromise = new Promise((resolve) => {
      resolveFirstBlockFromZMQPromise = resolve;
    });

    coreZMQClient.on(ZMQClient.TOPICS.hashblock, async (blockHash) => {
      const { result: block } = await coreRpcClient.getBlock(blockHash);
      const socketChainLock = new ChainLock({
        height: block.height,
        blockHash,
        signature,
      });

      latestCoreChainLock.update(socketChainLock);

      logger.debug(socketChainLock.toJSON(), `Updated fake latestCoreChainLock for height ${block.height}`);

      if (resolveFirstBlockFromZMQPromise) {
        resolveFirstBlockFromZMQPromise();
        resolveFirstBlockFromZMQPromise = null;
      }
    });

    const { result: rpcBestBlockHash } = await coreRpcClient.getBestBlockHash();
    const { result: rpcBestBlock } = await coreRpcClient.getBlock(rpcBestBlockHash);

    if (rpcBestBlock.height > 0) {
      const chainLock = new ChainLock({
        height: rpcBestBlock.height,
        blockHash: rpcBestBlockHash,
        signature,
      });

      latestCoreChainLock.update(chainLock);
    } else {
      // We need to wait for a new block from ZMQ socket
      logger.debug('There is no blocks currently. Waiting for a first one...');

      await firstBlockFromZMQPromise;
    }
  }

  return waitForCoreChainLockSyncFallback;
}

module.exports = waitForCoreChainLockSyncFallbackFactory;
