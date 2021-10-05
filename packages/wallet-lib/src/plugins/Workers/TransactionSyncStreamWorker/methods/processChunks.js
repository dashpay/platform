/* eslint-disable no-param-reassign */
const logger = require('../../../../logger');

function isAnyIntersection(arrayA, arrayB) {
  const intersection = arrayA.filter((e) => arrayB.indexOf(e) > -1);
  return intersection.length > 0;
}

async function processChunks(dataChunk) {
  const self = this;
  const addresses = this.getAddressesToSync();
  const { network } = this;
  /* First check if any instant locks appeared */
  const instantLocksReceived = this.constructor.getInstantSendLocksFromResponse(dataChunk);
  instantLocksReceived.forEach((isLock) => {
    this.importInstantLock(isLock);
  });

  /* Incoming transactions handling */
  const transactionsFromResponse = this.constructor
    .getTransactionListFromStreamResponse(dataChunk);

  const walletTransactions = this.constructor
    .filterWalletTransactions(transactionsFromResponse, addresses, network);

  if (walletTransactions.transactions.length) {
    // When a transaction exist, there is multiple things we need to do :
    // 1) The transaction itself needs to be imported
    const addressesGeneratedCount = await self
      .importTransactions(walletTransactions.transactions);

    // 2) Transaction metadata need to be fetched and imported as well.
    //    as such event might happen in the future
    //    As we require height information, we fetch transaction using client

    const awaitingMetadataPromises = walletTransactions.transactions
      .map((transaction) => self.handleTransactionFromStream(transaction)
        .then(({
          transactionResponse,
          metadata,
        }) => [transactionResponse.transaction, metadata]));

    Promise
      .all(awaitingMetadataPromises)
      .then(async (transactionsWithMetadata) => {
        await self.importTransactions(transactionsWithMetadata);
      });

    self.hasReachedGapLimit = self.hasReachedGapLimit || addressesGeneratedCount > 0;

    if (self.hasReachedGapLimit && self.stream) {
      logger.silly('TransactionSyncStreamWorker - end stream - new addresses generated');
      // If there are some new addresses being imported
      // to the storage, that mean that we hit the gap limit
      // and we need to update the bloom filter with new addresses,
      // i.e. we need to open another stream with a bloom filter
      // that contains new addresses.

      // DO not setting null this.stream allow to know we
      // need to reset our stream (as we pass along the error)
      // Wrapping `cancel` in `setImmediate` due to bug with double-free
      // explained here (https://github.com/grpc/grpc-node/issues/1652)
      // and here (https://github.com/nodejs/node/issues/38964)
      await new Promise((resolveCancel) => setImmediate(() => {
        self.stream.cancel();
        resolveCancel();
      }));
    }
  }

  /* Incoming Merkle block handling */
  const merkleBlockFromResponse = this.constructor
    .getMerkleBlockFromStreamResponse(dataChunk);

  if (merkleBlockFromResponse) {
    // Reverse hashes, as they're little endian in the header
    const transactionsInHeader = merkleBlockFromResponse.hashes.map((hashHex) => Buffer.from(hashHex, 'hex').reverse().toString('hex'));
    const transactionsInWallet = Object.keys(self.storage.getStore().transactions);
    const isTruePositive = isAnyIntersection(transactionsInHeader, transactionsInWallet);
    if (isTruePositive) {
      self.importBlockHeader(merkleBlockFromResponse.header);
    }
  }
}

module.exports = processChunks;
