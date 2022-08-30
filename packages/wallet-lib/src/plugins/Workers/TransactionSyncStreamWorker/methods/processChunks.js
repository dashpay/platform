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
      // if (this.transactionsToVerify[tx.hash]) {
      //   throw new Error(`Transaction ${tx.hash} already sits in verification queue`);
      // }
      this.transactionsToVerify[tx.hash] = tx;
    });

    if (syncIncomingTransactions && walletTransactions.length) {
      // Immediately import unconfirmed transactions to proceed with the broadcasting and etc
      // TODO: I guess they should be first confirmed by the instant locks
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
    throw new Error('Header metadata was not found during the merkle block processing');
  }

  const headerHeight = headerMetadata.height;
  const headerTime = headerMetadata.time;

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

  // TODO: verify merkle block

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
