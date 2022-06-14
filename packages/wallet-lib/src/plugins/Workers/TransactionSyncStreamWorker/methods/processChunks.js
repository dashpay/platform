/* eslint-disable no-param-reassign */
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const logger = require('../../../../logger');
const isBrowser = require('../../../../utils/isBrowser');

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

  const addressesTransaction = this.constructor
    .filterAddressesTransactions(transactionsFromResponse, addresses, network);

  /* Incoming Merkle block handling */
  const merkleBlock = this.constructor
    .getMerkleBlockFromStreamResponse(dataChunk);

  if (merkleBlock) {
    // Reverse hashes, as they're little endian in the header
    const txHashesInTheBlock = merkleBlock
      .hashes.reduce((set, hashHex) => {
        // const hash = Buffer.from(hashHex, 'hex').reverse().toString('hex');
        const hash = Buffer.from(hashHex, 'hex');
        hash.reverse();
        set.add(hash.toString('hex'));
        return set;
      }, new Set());

    this.chainSyncMediator.txChunkHashes.forEach((hash) => {
      if (!txHashesInTheBlock.has(hash)) {
        console.log('Alarm! prev chunk hashes are not present in the merkle block');
      }
      this.chainSyncMediator.transactionsBlockHashes[hash] = merkleBlock.header.hash;
    });
    this.chainSyncMediator.lastSyncedMerkleBlockHash = merkleBlock.header.hash;
    const lastSyncedHeaderHeight = this.chainSyncMediator.updateProgress(this.parentEvents);

    // TODO: attention, might be a temporary construction
    const walletStore = this.storage.getWalletStore(this.walletId);
    walletStore.updateLastKnownBlock(lastSyncedHeaderHeight);
    this.storage.scheduleStateSave();

    this.chainSyncMediator.txChunkHashes.clear();
    // console.log('[processChunks] Import merkle block for txs!', txHashesInTheBlock);
  }

  // console.log('Txs from stream',
  // transactionsFromResponse.length, addressesTransaction.transactions.length);
  if (addressesTransaction.transactions.length) {
    addressesTransaction.transactions.forEach((tx) => {
      const { hash } = tx;
      if (this.chainSyncMediator.txChunkHashes.has(hash)) {
        console.warn('Duplicated TX from the stream!!!', hash);
      }

      this.chainSyncMediator.txChunkHashes.add(hash);
    });

    // Normalizing format of transaction for account.importTransactions
    const addressesTransactionsWithoutMetadata = addressesTransaction.transactions
      .map((tx) => [tx]);

    // When a transaction exist, there is multiple things we need to do :
    // 1) The transaction itself needs to be imported
    const { addressesGenerated: addressesGeneratedCount } = await self
      .importTransactions(addressesTransactionsWithoutMetadata);
    // 2) Transaction metadata need to be fetched and imported as well.
    //    as such event might happen in the future
    //    As we require height information, we fetch transaction using client

    const awaitingMetadataPromises = addressesTransaction.transactions
      .map((transaction) => self.handleTransactionFromStream(transaction)
        .then(({
          transactionResponse,
          metadata,
        }) => [transactionResponse.transaction, metadata]));

    Promise
      .all(awaitingMetadataPromises)
      .then(async (transactionsWithMetadata) => {
        // Import into account
        const { mostRecentHeight } = await self.importTransactions(transactionsWithMetadata);

        if (mostRecentHeight !== -1) {
          // TODO: remove 'true', it now has to be covered by the ChainSyncMediator
          this.setLastSyncedBlockHeight(mostRecentHeight);
        }

        // Schedule save state after all chain data has been imported
        this.storage.scheduleStateSave();
      })
      .catch((err) => {
        logger.error('Error while importing transactions', err);
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
