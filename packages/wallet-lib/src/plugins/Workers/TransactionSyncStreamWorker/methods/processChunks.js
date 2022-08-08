/* eslint-disable no-param-reassign */
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const logger = require('../../../../logger');
const isBrowser = require('../../../../utils/isBrowser');

function processInstantLocks(instantLocks) {
  instantLocks.forEach((isLock) => {
    this.importInstantLock(isLock);
  });
}

function processTransactions(transactions) {
  const { network, syncIncomingTransactions } = this;
  const addresses = this.getAddressesToSync();

  const { transactions: walletTransactions } = this.constructor
    .filterAddressesTransactions(transactions, addresses, network);
  if (walletTransactions.length) {
    walletTransactions.forEach((tx) => {
      if (this.transactionsToVerify[tx.hash]) {
        console.warn(`!!! [processChunks] Duplicate tx: ${tx.hash}`);
      }
      this.transactionsToVerify[tx.hash] = tx;
    });

    if (syncIncomingTransactions && walletTransactions.length) {
      // Immediately import unconfirmed transactions to proceed with the broadcasting and etc
      // I guess they should be first confirmed by the instant locks)

      // TODO: reconnect to the stream if new addresses were generated
      this.importTransactions(walletTransactions.map((tx) => [tx]));
    }
  }
}

async function processMerkleBlock(merkleBlock) {
  const chainStore = this.storage.getDefaultChainStore();

  // Reverse hashes, as they're little endian in the header
  const txHashesInTheBlock = merkleBlock
    .hashes.reduce((set, hashHex) => {
      const hash = Buffer.from(hashHex, 'hex');
      hash.reverse();
      set.add(hash.toString('hex'));
      return set;
    }, new Set());

  const headerHash = merkleBlock.header.hash;
  const headerMetadata = chainStore.state.headersMetadata.get(headerHash);

  if (!headerMetadata) {
    throw new Error('Header metadata not found during the merkle block processing');
  }

  const headerHeight = headerMetadata.height;
  const headerTime = headerMetadata.time;

  if (!headerTime || !headerHeight) {
    throw new Error(`Invalid header metadata: Time: ${headerTime}, Height: ${headerHeight}`);
  }

  const metadata = {
    blockHash: headerHash,
    height: headerHeight,
    time: headerTime,
    instantLocked: false, // TBD
    chainLocked: false, // TBD
  };

  const transactionsWithMetadata = [];
  Object.keys(this.transactionsToVerify).forEach((hash) => {
    const tx = this.transactionsToVerify[hash];
    if (!txHashesInTheBlock.has(hash)) {
      throw new Error(`Transaction ${hash} was not found in merkle block ${headerHash}`);
    }
    transactionsWithMetadata.push([tx, metadata]);
    delete this.transactionsToVerify[hash];
  });

  // TODO: verify merkle block

  let addressesGenerated = 0;
  if (transactionsWithMetadata.length) {
    ({ addressesGenerated } = this.importTransactions(transactionsWithMetadata));
  }

  this.setLastSyncedBlockHeight(headerHeight, true);
  this.scheduleProgressUpdate();

  // TODO: test restart
  // Close current stream, so that new one could be re-created with more addresses in bloom filter
  if (addressesGenerated > 0) {
    logger.silly('TransactionSyncStreamWorker - end stream - new addresses generated');

    if (isBrowser()) {
      // Under browser environment, grpc-web doesn't call error and end events
      // so we call it by ourselves
      await new Promise((resolveCancel) => setImmediate(() => {
        this.stream.cancel();
        const error = new GrpcError(GrpcErrorCodes.CANCELLED, 'Cancelled on client');

        // call onError events
        this.stream.f.forEach((func) => func(error));

        // call onEnd events
        this.stream.c.forEach((func) => func());
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
        this.stream.cancel();
        resolveCancel();
      }));
    }
  }
}

async function processChunks(dataChunk) {
  const instantLocks = this.constructor
    .getInstantSendLocksFromResponse(dataChunk);
  const transactions = this.constructor
    .getTransactionListFromStreamResponse(dataChunk);
  const merkleBlock = this.constructor
    .getMerkleBlockFromStreamResponse(dataChunk);

  if (instantLocks.length) {
    processInstantLocks.bind(this)(instantLocks);
  } if (transactions.length) {
    processTransactions.bind(this)(transactions);
  } else if (merkleBlock) {
    await processMerkleBlock.bind(this)(merkleBlock);
  } else {
    logger.debug('[processChunk] TX Stream data chunk has not been recognized');
  }
}

module.exports = processChunks;
