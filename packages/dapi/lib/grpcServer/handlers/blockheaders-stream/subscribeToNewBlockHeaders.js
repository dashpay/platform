const { BlockHeader } = require('@dashevo/dashcore-lib');
const ChainLockSigMessage = require('@dashevo/dashcore-lib/lib/zmqMessages/ChainLockSigMessage');
const ProcessMediator = require('./ProcessMediator');
const wait = require('../../../utils/wait');

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
  let latestClSig = null;

  let isClientConnected = true;

  /**
   * @param {Buffer} hash
   */
  const blockHashHandler = (hash) => {
    cachedHeadersHashes.add(hash.toString('hex'));
  };

  /**
   * @param {Buffer} rawClSigMessage
   */
  const rawClSigHandler = (rawClSigMessage) => {
    const { chainLock } = new ChainLockSigMessage(rawClSigMessage);
    latestClSig = chainLock.signature;
  };

  zmqClient.on(
    zmqClient.topics.hashblock,
    blockHashHandler,
  );

  zmqClient.on(
    zmqClient.topics.rawchainlocksig,
    rawClSigHandler,
  );

  mediator.on(ProcessMediator.EVENTS.HISTORICAL_BLOCK_HEADERS_SENT, (hashes) => {
    // Remove data from cache by hashes
    hashes.forEach((hash) => {
      cachedHeadersHashes.delete(hash);
    });
  });

  // Receive an event when all historical data is sent to the user.
  mediator.once(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT, async () => {
    // Run a loop until client is disconnected and send cached as well
    // as new data (through the cache) continuously after that.
    // Cache is populated from ZMQ events.
    while (isClientConnected) {
      if (cachedHeadersHashes.size) {
        // TODO: figure out whether it's possible to omit BlockHeader.fromBuffer conversion
        // and directly send bytes to the client
        const blockHeaders = await Promise.all(Array.from(cachedHeadersHashes)
          .map(async (hash) => BlockHeader.fromBuffer(await coreAPI.getBlockHeader(hash))));

        mediator.emit(ProcessMediator.EVENTS.BLOCK_HEADERS, blockHeaders);
        cachedHeadersHashes.clear();
      }

      if (latestClSig) {
        mediator.emit(ProcessMediator.EVENTS.CHAIN_LOCK_SIGNATURE, latestClSig);
        latestClSig = null;
      }

      // TODO: pick a right time interval having in mind that issuance of the block headers
      // is not frequent
      await wait(5000);
    }
  });

  mediator.once(ProcessMediator.EVENTS.CLIENT_DISCONNECTED, () => {
    isClientConnected = false;
    mediator.removeAllListeners();
    zmqClient.removeListener(zmqClient.topics.hashblock, blockHashHandler);
    zmqClient.removeListener(zmqClient.topics.rawchainlocksig, rawClSigHandler);
  });
}

module.exports = subscribeToNewBlockHeaders;
