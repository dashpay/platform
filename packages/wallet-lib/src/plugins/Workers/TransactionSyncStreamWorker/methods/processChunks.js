/* eslint-disable no-param-reassign */
const logger = require('../../../../logger');

function processInstantLocks(instantLocks) {
  instantLocks.forEach((isLock) => {
    this.importInstantLock(isLock);
  });
}

async function processTransactions(transactions) {
  const { network, syncIncomingTransactions } = this;
  const addresses = this.getAddressesToSync();

  const { transactions: walletTransactions } = this.constructor
    .filterAddressesTransactions(transactions, addresses, network);
  if (walletTransactions.length) {
    walletTransactions.forEach((tx) => {
      // TODO(spv): fine tune this behaviour for the cases where
      // after TX stream reconnected, we are obtaining the same transactions that
      // already in this.transactionsToVerify

      // if (this.transactionsToVerify[tx.hash]) {
      //   throw new Error(`Transaction ${tx.hash} already sits in verification queue`);
      // }
      this.transactionsToVerify[tx.hash] = tx;
    });

    if (syncIncomingTransactions && walletTransactions.length) {
      // Immediately import unconfirmed transactions to proceed with the broadcasting and etc

      // TODO(spv): I guess they should be first confirmed by the instant locks,
      //  but this functionality was not implemented properly up to this date

      const {
        addressesGenerated,
        mostRecentHeight,
      } = this.importTransactions(walletTransactions.map((tx) => [tx]));
      this.setLastSyncedBlockHeight(mostRecentHeight, true);

      if (addressesGenerated > 0) {
        this.reconnectOnNewBlock = true;
      }
    }
  }
}

async function processMerkleBlock(merkleBlock) {
  // Ignore Merkle Block processing during the incoming sync
  // because transactions are verified by actual blocks (handleNewBlock)
  //
  // Overall, merkleblocks for freshly mined transactions aren't arriving at all during
  // the continuous TX sync. (Perhaps due to a bug in dapi?)
  // However, the stream returns them for blocks N where
  // subscribeToTransactionsWithProofs({ fromBlockHeight: N, count: 0 })
  //
  // This condition ignores such merkle blocks
  if (this.syncIncomingTransactions) {
    return;
  }

  const chainStore = this.storage.getDefaultChainStore();

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
    throw new Error('Header metadata was not found during the merkle block processing');
  }

  const headerHeight = headerMetadata.height;
  const headerTime = headerMetadata.time;

  logger.silly(
    `[TransactionSyncStreamWorker] processing merkle block ${headerHash} at height ${headerHeight}`,
    { walletId: this.walletId },
  );

  if (!headerTime || !headerHeight) {
    throw new Error(`Invalid header metadata: Time: ${headerTime}, Height: ${headerHeight}`);
  }

  const metadata = {
    blockHash: headerHash,
    height: headerHeight,
    time: new Date(headerTime * 1e3),
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

  // TODO(spv): verify transactions agaoinst the merkle block

  let addressesGenerated = 0;
  if (transactionsWithMetadata.length) {
    ({ addressesGenerated } = this.importTransactions(transactionsWithMetadata));
  }

  this.setLastSyncedBlockHeight(headerHeight, true);
  this.scheduleProgressUpdate();

  if (addressesGenerated > 0) {
    await this.reconnectToStream();
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
  } else if (transactions.length) {
    await processTransactions.bind(this)(transactions);
  } else if (merkleBlock) {
    await processMerkleBlock.bind(this)(merkleBlock);
  } else {
    logger.debug('[processChunk] TX Stream data chunk has not been recognized');
  }
}

module.exports = processChunks;
