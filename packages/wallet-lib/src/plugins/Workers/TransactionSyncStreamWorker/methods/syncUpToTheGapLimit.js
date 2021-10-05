const logger = require('../../../../logger');
const onStreamEnd = require('../handlers/onStreamEnd');
const onStreamError = require('../handlers/onStreamError');
const onStreamData = require('../handlers/onStreamData');
const Queue = require('../../../../utils/Queue/Queue');
/**
 *
 * @param options
 * @param {string} [options.fromBlockHash]
 * @param {number} count
 * @param {string} network
 * @param {number} [options.fromBlockHeight]
 * @return {Promise<undefined>}
 */
module.exports = async function syncUpToTheGapLimit({
  fromBlockHash, count, network, fromBlockHeight,
}) {
  const self = this;
  const addresses = this.getAddressesToSync();
  self.addresses = addresses;
  logger.debug(`syncing up to the gap limit: - from block: ${fromBlockHash || fromBlockHeight} Count: ${count}`);

  if (fromBlockHash == null && fromBlockHeight == null) {
    throw new Error('fromBlockHash ot fromBlockHeight should be present');
  }

  const options = { count };
  if (fromBlockHash != null) {
    options.fromBlockHash = fromBlockHash;
  } else {
    options.fromBlockHeight = fromBlockHeight;
  }

  const stream = await this.transport
    .subscribeToTransactionsWithProofs(addresses, options);

  if (self.stream) {
    throw new Error('Limited to one stream at the same time.');
  }
  self.stream = stream;
  self.network = network;
  self.hasReachedGapLimit = false;
  // The order is important, however, some async calls are being performed
  // in order to additionnaly fetch metadata for each valid tx chunks.
  // We therefore need to temporarily store chunks for handling.
  self.chunksQueue = new Queue();

  // eslint-disable-next-line no-async-promise-executor
  return new Promise(async (resolve, reject) => {
    // handler for error being thrown for job processing
    self.chunksQueue.on('error', (error) => {
      reject(error);
    });
    stream
      .on('data', (data) => onStreamData(self, data))
      .on('error', (error) => onStreamError(error, reject))
      .on('end', () => onStreamEnd(self, resolve));
  });
};
