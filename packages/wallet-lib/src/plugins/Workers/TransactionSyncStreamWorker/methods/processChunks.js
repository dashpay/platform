/* eslint-disable no-param-reassign */
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const logger = require('../../../../logger');
const isBrowser = require('../../../../utils/isBrowser');
const EVENTS = require('../../../../EVENTS');

function processInstantLocks(instantLocks) {
  instantLocks.forEach((isLock) => {
    this.importInstantLock(isLock);
  });
}

const transactionsToVerify = {};
async function processTransactions(transactions) {
  const { network, syncIncomingTransactions } = this;
  const addresses = this.getAddressesToSync();

  const { transactions: walletTransactions } = this.constructor
    .filterAddressesTransactions(transactions, addresses, network);
  if (walletTransactions.length) {
    walletTransactions.forEach((tx) => {
      if (transactionsToVerify[tx.hash]) {
        console.warn(`!!! [processChunks] Duplicate tx: ${tx.hash}`);
      }
      transactionsToVerify[tx.hash] = tx;
    });

    if (syncIncomingTransactions && walletTransactions.length) {
      // Immediately import unconfirmed transactions

      // TODO: handle stream reconnect if addresses were generated
      this.importTransactions(walletTransactions.map((tx) => [tx]));

      // TODO: test and make sure it works
      await new Promise((resolve) => {
        const heightChangeListener = () => {
          Promise
            .all(walletTransactions.map((tx) => this.transport.getTransaction(tx.hash)))
            .then((result) => {
              const hasMetadata = result.every(({ blockHash }) => blockHash);
              if (hasMetadata) {
                const transactionsWithMetadata = result.map((item) => {
                  const {
                    transaction, blockHash, height, instantLocked, chainLocked,
                  } = item;
                  const metadata = {
                    blockHash,
                    height,
                    instantLocked,
                    chainLocked,
                  };
                  return [transaction, metadata];
                });

                this.importTransactions(transactionsWithMetadata);
                this.parentEvents.removeListener(EVENTS.BLOCKHEIGHT_CHANGED, heightChangeListener);
                resolve();
              }
            });
        };

        this.parentEvents.on(EVENTS.BLOCKHEIGHT_CHANGED, heightChangeListener);
      });
    }
  }
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
  Object.keys(transactionsToVerify).forEach((hash) => {
    const tx = transactionsToVerify[hash];
    if (!txHashesInTheBlock.has(hash)) {
      throw new Error(`Transaction ${hash} was not found in merkle block ${headerHash}`);
    }
    transactionsWithMetadata.push([tx, metadata]);
    delete transactionsToVerify[hash];
  });

  // TODO: verify merkle block

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
    await processTransactions.bind(this)(transactions);
  } else if (merkleBlock) {
    await processMerkleBlock.bind(this)(merkleBlock);
  } else {
    logger.debug('[processChunk] TX Stream data chunk has not been recognized');
  }
}

module.exports = processChunks;
