const logger = require('../../../../logger');

function isAnyIntersection(arrayA, arrayB) {
  const intersection = arrayA.filter((e) => arrayB.indexOf(e) > -1);
  return intersection.length > 0;
}

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
  let reachedGapLimit = false;

  // eslint-disable-next-line no-async-promise-executor
  return new Promise(async (resolve, reject) => {
    stream
      .on('data', async (response) => {
        /* First check if any instant locks appeared */
        const instantLocksReceived = this.constructor.getInstantSendLocksFromResponse(response);
        instantLocksReceived.forEach((isLock) => {
          this.importInstantLock(isLock);
        });

        /* Incoming transactions handling */
        const transactionsFromResponse = this.constructor
          .getTransactionListFromStreamResponse(response);
        const walletTransactions = this.constructor
          .filterWalletTransactions(transactionsFromResponse, addresses, network);

        if (walletTransactions.transactions.length) {
          const transactionsWithMetadata = [];
          // As we require height information, we fetch transaction using client.
          // eslint-disable-next-line no-restricted-syntax
          for (const transaction of walletTransactions.transactions) {
            // eslint-disable-next-line no-underscore-dangle
            const transactionHash = transaction._getHash().reverse().toString('hex');
            // eslint-disable-next-line no-await-in-loop
            const getTransactionResponse = await this.transport.getTransaction(transactionHash);
            const metadata = {
              blockHash: getTransactionResponse.blockHash,
              height: getTransactionResponse.height,
              instantLocked: getTransactionResponse.instantLocked,
              chainLocked: getTransactionResponse.chainLocked,
            };
            transactionsWithMetadata.push([getTransactionResponse.transaction, metadata]);
          }

          const addressesGeneratedCount = await self
            .importTransactions(transactionsWithMetadata);

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

        /* Incoming Merkle block handling */
        const merkleBlockFromResponse = this.constructor
          .getMerkleBlockFromStreamResponse(response);

        if (merkleBlockFromResponse) {
          // Reverse hashes, as they're little endian in the header
          const transactionsInHeader = merkleBlockFromResponse.hashes.map((hashHex) => Buffer.from(hashHex, 'hex').reverse().toString('hex'));
          const transactionsInWallet = Object.keys(self.storage.getStore().transactions);
          const isTruePositive = isAnyIntersection(transactionsInHeader, transactionsInWallet);
          if (isTruePositive) {
            self.importBlockHeader(merkleBlockFromResponse.header);
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
