const { BlockHeader } = require('@dashevo/dashcore-lib');
const ProcessMediator = require('../../../transactionsFilter/ProcessMediator');
const wait = require('../../../utils/wait');

// TODO: could we have this file inside blockheaders-stream directory
// and not pollute transactionsFilter directory?

/**
 * @typedef subscribeToNewTransactions
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

  let isClientConnected = true;
  let blocksFromZmq = 0;

  /**
   * @param {Buffer} hash
   */
  const blockHashHandler = (hash) => {
    blocksFromZmq++;
    cachedHeadersHashes.add(hash.toString('hex'));
    console.log('Add to cache from zmq', hash.toString('hex'));
    console.log(`Obtained ${blocksFromZmq} block from ZMQ`);
  };

  zmqClient.on(
    zmqClient.topics.hashblock,
    blockHashHandler,
  );

  let totalHistoricalHeadersSent = 0;
  mediator.on(ProcessMediator.EVENTS.HISTORICAL_BLOCK_HEADERS_SENT, (hashes) => {
    totalHistoricalHeadersSent += hashes.length;
    console.log(`Sent total ${totalHistoricalHeadersSent} historical headers`);
    // Remove data from cache by hashes
    console.log('Bef', cachedHeadersHashes.size);
    hashes.forEach((hash) => {
      cachedHeadersHashes.delete(hash);
    });
    console.log('Aft', cachedHeadersHashes.size);
  });

  // Receive an event when all historical data is sent to the user.
  mediator.once(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT, async () => {
    console.log('All historical data has been sent. Start processing new data.');
    // Run a loop until client is disconnected and send cached as well
    // as new data (through the cache) continuously after that.
    // Cache is populated from ZMQ events.
    while (isClientConnected) {
      if (cachedHeadersHashes.size) {
        const blockHeaders = await Promise.all(Array.from(cachedHeadersHashes)
          .map(async (hash) => BlockHeader.fromRawBlock(await coreAPI.getBlockHeader(hash))));

        blockHeaders.sort((a, b) => a.timestamp - b.timestamp);
        mediator.emit(ProcessMediator.EVENTS.BLOCK_HEADERS, blockHeaders);
        console.log(`Send ${blockHeaders.length} block headers to client`);
        cachedHeadersHashes.clear();
      }

      // TODO: pick a right time interval having in mind that issuance of the block headers
      // is not frequent
      await wait(5000);
    }
  });

  mediator.once(ProcessMediator.EVENTS.CLIENT_DISCONNECTED, () => {
    console.log('Finish stream');
    isClientConnected = false;
    mediator.removeAllListeners();
    zmqClient.removeListener(zmqClient.topics.hashblock, blockHashHandler);
  });
}

module.exports = subscribeToNewBlockHeaders;
