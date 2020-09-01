const logger = require('../../../../logger');

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
  logger.debug(`syncing up to the gap limit: - fromBlockHeight: ${fromBlockHash || fromBlockHeight} Count: ${count}`);

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
  let reachedGapLimit = false;

  return new Promise((resolve, reject) => {
    stream
      .on('data', async (response) => {
        const merkleBlockFromResponse = this.constructor
          .getMerkleBlockFromStreamResponse(response);

        if (merkleBlockFromResponse) {
          self.importBlockHeader(merkleBlockFromResponse.header);
        }

        const transactionsFromResponse = this.constructor
          .getTransactionListFromStreamResponse(response);
        const walletTransactions = this.constructor
          .filterWalletTransactions(transactionsFromResponse, addresses, network);

        if (walletTransactions.transactions.length) {
          const addressesGeneratedCount = await self
            .importTransactions(walletTransactions.transactions);

          reachedGapLimit = reachedGapLimit || addressesGeneratedCount > 0;

          if (reachedGapLimit) {
            logger.silly('TransactionSyncStreamWorker - end stream - new addresses generated');
            // If there are some new addresses being imported
            // to the storage, that mean that we hit the gap limit
            // and we need to update the bloom filter with new addresses,
            // i.e. we need to open another stream with a bloom filter
            // that contains new addresses.

            // DO not setting null this.stream allow to know we
            // need to reset our stream (as we pass along the error)
            stream.cancel();
          }
        }
      })
      .on('error', (err) => {
        logger.silly('TransactionSyncStreamWorker - end stream on error');
        reject(err);
      })
      .on('end', () => {
        logger.silly('TransactionSyncStreamWorker - end stream on request');
        self.stream = null;
        resolve(reachedGapLimit);
      });
  });
};
