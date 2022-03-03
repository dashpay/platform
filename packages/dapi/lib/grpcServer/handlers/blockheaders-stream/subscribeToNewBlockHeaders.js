const { BlockHeader } = require('@dashevo/dashcore-lib');
const ProcessMediator = require('./ProcessMediator');
const wait = require('../../../utils/wait');
const { NEW_BLOCK_HEADERS_PROPAGATE_INTERVAL } = require('./constants');

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
   * @param {string} blockHash
   */
  const blockHashHandler = (blockHash) => {
    pendingHeadersHashes.add(blockHash);
  };

  /**
   *
   * @param chainLock {ChainLock}
   */
  const chainLockHandler = (chainLock) => {
    lastChainLock = chainLock;
  };

  chainDataProvider.on(chainDataProvider.events.NEW_BLOCK_HEADER, blockHashHandler);
  chainDataProvider.on(chainDataProvider.events.NEW_CHAIN_LOCK, chainLockHandler);

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
            const rawBlockHeader = await chainDataProvider.getBlockHeader(hash);
            return new BlockHeader(Buffer.from(rawBlockHeader, 'hex'));
          }));

        mediator.emit(ProcessMediator.EVENTS.BLOCK_HEADERS, blockHeaders);
        pendingHeadersHashes.clear();
      }

      if (lastChainLock) {
        mediator.emit(ProcessMediator.EVENTS.CHAIN_LOCK, lastChainLock);
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
    chainDataProvider.removeListener(chainDataProvider.events.NEW_BLOCK_HEADER, blockHashHandler);
    chainDataProvider.removeListener(chainDataProvider.events.NEW_CHAIN_LOCK, chainLockHandler);
  });
}

module.exports = subscribeToNewBlockHeaders;
