const { BlockHeader, ChainLock } = require('@dashevo/dashcore-lib');
const ProcessMediator = require('./ProcessMediator');
const wait = require('../../../utils/wait');
const { NEW_BLOCK_HEADERS_PROPAGATE_INTERVAL } = require('./constants');
const cache = require('../core/cache')

/**
 * @typedef subscribeToNewBlockHeaders
 * @param {ProcessMediator} mediator
 * @param {CoreRpcClient} coreAPI
 * @param {ZmqClient} zmqClient
 */
function subscribeToNewBlockHeaders(
  mediator,
  zmqClient,
  coreAPI,
) {
  const cachedHeadersHashes = new Set();
  let latestChainLock = null;

  let isClientConnected = true;

  /**
   * @param {Buffer} hash
   */
  const blockHashHandler = (hash) => {
    cachedHeadersHashes.add(hash.toString('hex'));
  };

  /**
   * @param {Buffer} rawChainLock
   */
  const rawChainLockHandler = (rawChainLock) => {
    latestChainLock = new ChainLock(rawChainLock);
  };

  zmqClient.on(
    zmqClient.topics.hashblock,
    blockHashHandler,
  );

  zmqClient.on(
    zmqClient.topics.rawchainlock,
    rawChainLockHandler,
  );

  mediator.on(ProcessMediator.EVENTS.HISTORICAL_BLOCK_HEADERS_SENT, (hashes) => {
    // Remove data from cache by hashes
    hashes.forEach((hash) => {
      cachedHeadersHashes.delete(hash);
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
      if (cachedHeadersHashes.size) {
        // TODO: figure out whether it's possible to omit new BlockHeader() conversion
        // and directly send bytes to the client
        const blockHeaders = await Promise.all(Array.from(cachedHeadersHashes)
          .map(async (hash) => {
            const cachedBlockHeader = cache.get(hash)

            if (!cachedBlockHeader) {
              const rawBlockHeader = await coreAPI.getBlockHeader(hash);
              return new BlockHeader(Buffer.from(rawBlockHeader, 'hex'));
            }

            return cachedBlockHeader
          }));

        mediator.emit(ProcessMediator.EVENTS.BLOCK_HEADERS, blockHeaders);
        cachedHeadersHashes.clear();
      }

      if (latestChainLock) {
        mediator.emit(ProcessMediator.EVENTS.CHAIN_LOCK, latestChainLock);
        latestChainLock = null;
      }

      // TODO: pick a right time interval having in mind that issuance of the block headers
      // is not frequent
      await wait(NEW_BLOCK_HEADERS_PROPAGATE_INTERVAL);
    }
  });

  mediator.once(ProcessMediator.EVENTS.CLIENT_DISCONNECTED, () => {
    isClientConnected = false;
    mediator.removeAllListeners();
    zmqClient.removeListener(zmqClient.topics.hashblock, blockHashHandler);
    zmqClient.removeListener(zmqClient.topics.rawchainlock, rawChainLockHandler);
  });
}

module.exports = subscribeToNewBlockHeaders;
