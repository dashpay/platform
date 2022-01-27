/* eslint-disable no-param-reassign */
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const logger = require('../../../../logger');
const isBrowser = require('../../../../utils/isBrowser');
const TempChainCache = require('../TempChainCache');

function getIntersection(arrayA, arrayB) {
  return arrayA.filter((e) => arrayB.indexOf(e) > -1);
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

  /* Incoming Merkle block handling */
  const merkleBlockFromResponse = this.constructor
    .getMerkleBlockFromStreamResponse(dataChunk);

  if (merkleBlockFromResponse) {
    // Reverse hashes, as they're little endian in the header
    const transactionsInHeader = merkleBlockFromResponse.hashes.map((hashHex) => Buffer.from(hashHex, 'hex').reverse().toString('hex'));
    const transactionsInWallet = Object.keys(self.storage.getStore().transactions);
    const intersection = getIntersection(transactionsInHeader, transactionsInWallet);
    if (intersection.length) {
      const { header } = merkleBlockFromResponse;
      self.importBlockHeader(header);

      if (TempChainCache.i().transactionsByBlockHash[header.hash]) {
        console.log('Dup of', header.hash);
      }
      TempChainCache.i().transactionsByBlockHash[header.hash] = intersection;
    }
  }

  console.log('Wall', walletTransactions.transactions.length);
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

      if (isBrowser()) {
        // Under browser environment, grpc-web doesn't call error and end events
        // so we call it by ourselves
        await new Promise((resolveCancel) => setImmediate(() => {
          self.stream.cancel();
          const error = new GrpcError(GrpcErrorCodes.CANCELLED, 'Cancelled on client');

          // call onError events
          self.stream.f.forEach((func) => func(error));

          // call onEnd events
          self.stream.c.forEach((func) => func());
          resolveCancel();
        }));
      } else {
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
  }
}

module.exports = processChunks;
