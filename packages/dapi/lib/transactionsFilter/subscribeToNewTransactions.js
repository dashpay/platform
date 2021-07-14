const TransactionHashesCache = require('./TransactionHashesCache');
const BloomFilterEmitter = require('../bloomFilter/emitter/BloomFilterEmitter');

const ProcessMediator = require('../transactionsFilter/ProcessMediator');

const wait = require('../utils/wait');

/**
 * @typedef subscribeToNewTransactions
 * @param {ProcessMediator} mediator
 * @param {BloomFilter} filter
 * @param {testFunction} testTransactionAgainstFilter
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 */
function subscribeToNewTransactions(
  mediator,
  filter,
  testTransactionAgainstFilter,
  bloomFilterEmitterCollection,
) {
  const filterEmitter = new BloomFilterEmitter(filter, testTransactionAgainstFilter);

  const transactionsAndBlocksCache = new TransactionHashesCache();

  let isClientConnected = true;

  // store and emit transaction or a locked transaction hash when they match the bloom filter
  filterEmitter.on('match', (transaction) => {
    // Store the matched transaction
    // in order to build a merkle block with sent transactions
    transactionsAndBlocksCache.addTransaction(transaction);
  });

  // prepare and emit merkle block with previously sent transactions when they got mined
  filterEmitter.on('block', (block) => {
    // in case we've missed some or all transactions and got a block
    if (transactionsAndBlocksCache.getBlockCount() === 0) {
      // test transactions and emit `match` events
      block.transactions.forEach(tx => filterEmitter.test(tx));
    }

    // put block in the cache executing queue logic
    transactionsAndBlocksCache.addBlock(block);
  });

  // Collect instant locked transactions and locks
  // while we sending historical and mempool data
  const preMempoolSentInstantLockListener = ({ instantLock, transaction }) => {
    if (!testTransactionAgainstFilter(filter, transaction)) {
      return;
    }

    transactionsAndBlocksCache.addInstantLock(instantLock);
  };

  filterEmitter.on('instantLock', preMempoolSentInstantLockListener);

  // Receive an event when a historical block is sent to user,
  // so we can update our cache to an actual state,
  // removing transactions, blocks and merkle blocks from cache
  mediator.on(ProcessMediator.EVENTS.HISTORICAL_BLOCK_SENT, (blockHash) => {
    transactionsAndBlocksCache.removeDataByBlockHash(blockHash);
  });

  // Receive an event when all historical and mempool data (is any) is sent to the user.
  mediator.once(ProcessMediator.EVENTS.MEMPOOL_DATA_SENT, async () => {
    // When mempool transactions are sent we start to send
    // instant locks right away instead of collecting them
    filterEmitter.removeListener('instantLock', preMempoolSentInstantLockListener);

    filterEmitter.on('instantLock', ({ instantLock }) => {
      const isTransactionInWaitingList = transactionsAndBlocksCache
        .isInInstantLockCache(instantLock.txid);

      if (isTransactionInWaitingList) {
        transactionsAndBlocksCache
          .removeTransactionHashFromInstantSendLockWaitingList(instantLock.txid);
        mediator.emit(ProcessMediator.EVENTS.INSTANT_LOCK, instantLock);
      }
    });

    // Run a loop until client is disconnected and send cached as well
    // as new data (through the cache) continuously after that.
    // Cache is populated from ZMQ events.
    while (isClientConnected) {
      // TODO We can send multiple items to optimize throughput
      //  Proto messages already support that
      const unsentTransactions = transactionsAndBlocksCache.getUnretrievedTransactions();
      unsentTransactions
        .forEach(tx => mediator.emit(ProcessMediator.EVENTS.TRANSACTION, tx));

      const unretrievedInstantLocks = transactionsAndBlocksCache.getUnretrievedInstantLocks();
      unretrievedInstantLocks.forEach((instantLock) => {
        if (transactionsAndBlocksCache.isInInstantLockCache(instantLock.txid)) {
          mediator.emit(ProcessMediator.EVENTS.INSTANT_LOCK, instantLock);
        }
      });

      const unsentMerkleBlocks = transactionsAndBlocksCache.getUnretrievedMerkleBlocks();
      unsentMerkleBlocks
        .forEach(merkleBlock => mediator.emit(ProcessMediator.EVENTS.MERKLE_BLOCK, merkleBlock));

      await wait(50);
    }
  });

  // Add the bloom filter emitter to the collection
  bloomFilterEmitterCollection.add(filterEmitter);

  mediator.once(ProcessMediator.EVENTS.CLIENT_DISCONNECTED, () => {
    isClientConnected = false;

    mediator.removeAllListeners();
    filterEmitter.removeAllListeners();

    bloomFilterEmitterCollection.remove(filterEmitter);
  });
}

module.exports = subscribeToNewTransactions;
