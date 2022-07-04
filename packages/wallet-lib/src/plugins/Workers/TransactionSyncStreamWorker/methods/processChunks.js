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

let transactionsToVerify = [];
function processTransactions(transactions) {
  const { network } = this;
  const addresses = this.getAddressesToSync();

  const { transactions: walletTransactions } = this.constructor
    .filterAddressesTransactions(transactions, addresses, network);
  transactionsToVerify = walletTransactions;
}

async function processMerkleBlock(merkleBlock) {
  const walletStore = this.storage.getWalletStore(this.walletId);

  // Reverse hashes, as they're little endian in the header
  const txHashesInTheBlock = merkleBlock
    .hashes.reduce((set, hashHex) => {
      const hash = Buffer.from(hashHex, 'hex');
      hash.reverse();
      set.add(hash.toString('hex'));
      return set;
    }, new Set());

  const { blockHeadersProvider } = this.transport.client;

  const headerHash = merkleBlock.header.hash;
  const headerHeight = blockHeadersProvider.spvChain.heightByHash[headerHash];

  const metadata = {
    blockHash: headerHash,
    height: headerHeight,
    instantLocked: false, // TBD
    chainLocked: false, // TBD
  };

  const transactionsWithMetadata = [];
  transactionsToVerify.forEach((tx) => {
    if (!txHashesInTheBlock.has(tx.hash)) {
      throw new Error(`Transaction ${tx.hash} was not found in merkle block ${headerHash}`);
    }
    transactionsWithMetadata.push([tx, metadata]);
  });

  // TODO: verify merkle block

  transactionsToVerify.splice(0);

  let addressesGenerated = 0;
  if (transactionsWithMetadata.length) {
    ({ addressesGenerated } = this.importTransactions(transactionsWithMetadata));
  }

  this.lastSyncedBlockHeight = headerHeight;
  walletStore.updateLastKnownBlock(headerHeight);
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
