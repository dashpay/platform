const { BlockHeader } = require('@dashevo/dashcore-lib');
const ProcessMediator = require('./ProcessMediator');
const wait = require('../../../utils/wait');
const { NEW_BLOCK_HEADERS_PROPAGATE_INTERVAL } = require('./constants');
const cache = require('../../../providers/blockheaders-cache');

/**
 * @typedef subscribeToNewBlockHeaders
 * @param {ProcessMediator} mediator
 * @param {ChainDataProvider} chainDataProvider
 */
function subscribeToNewBlockHeaders(mediator, chainDataProvider) {
  const pendingHeadersHashes = new Set();

  let lastChainLock;

  let isClientConnected = true;

  /**
   * @param {Buffer} hash
   */
  const blockHashHandler = (hash) => {
    pendingHeadersHashes.add(hash.toString('hex'));
  };

  /**
   *
   * @param chainLock {ChainLock}
   */
  const chainLockHandler = (chainLock) => {
    lastChainLock = chainLock;
  };

  chainDataProvider.on(chainDataProvider.events.newBlockHeader, blockHashHandler);
  chainDataProvider.on(chainDataProvider.events.newChainLock, chainLockHandler);

  mediator.on(ProcessMediator.EVENTS.HISTORICAL_BLOCK_HEADERS_SENT, (hashes) => {
    // Remove data from cache by hashes
    hashes.forEach((hash) => {
      pendingHeadersHashes.delete(hash);
    });
  });

  // Receive an event when all historical data is sent to the user.
  mediator.once(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT, async () => {
    // TODO: WARNING! If error is thrown within this function, it does not propagate
    // and do not fire UnhandledPromiseRejection

    // Run a loop until client is disconnected and send cached as well
    // as new data (through the cache) continuously after that.
    // Cache is populated from ZMQ events.
    while (isClientConnected) {
      if (pendingHeadersHashes.size) {
        // TODO: figure out whether it's possible to omit new BlockHeader() conversion
        // and directly send bytes to the client
        const blockHeaders = await Promise.all(Array.from(pendingHeadersHashes)
          .map(async (hash) => {
            const cachedBlockHeader = cache.get(hash);

            if (!cachedBlockHeader) {
              return chainDataProvider.getBlockHeader(hash);
            }

            return new BlockHeader(Buffer.from(cachedBlockHeader, 'hex'));
          }));

        mediator.emit(ProcessMediator.EVENTS.BLOCK_HEADERS, blockHeaders);
        pendingHeadersHashes.clear();
      }

      if (lastChainLock) {
        mediator.emit(ProcessMediator.EVENTS.CHAIN_LOCK, chainDataProvider.getBestChainLock());
        lastChainLock = null;
      }

      // TODO: pick a right time interval having in mind that issuance of the block headers
      // is not frequent
      await wait(NEW_BLOCK_HEADERS_PROPAGATE_INTERVAL);
    }
  });

  mediator.once(ProcessMediator.EVENTS.CLIENT_DISCONNECTED, () => {
    isClientConnected = false;
    mediator.removeAllListeners();
    chainDataProvider.removeListener(chainDataProvider.events.newBlockHeader, blockHashHandler);
    chainDataProvider.removeListener(chainDataProvider.events.newChainLock, chainLockHandler);
  });
}

module.exports = subscribeToNewBlockHeaders;
