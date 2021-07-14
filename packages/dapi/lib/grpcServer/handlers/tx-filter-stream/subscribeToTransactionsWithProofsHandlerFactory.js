const { BloomFilter } = require('@dashevo/dashcore-lib');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
    stream: {
      AcknowledgingWritable,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    TransactionsWithProofsResponse,
    RawTransactions,
    InstantSendLockMessages,
  },
} = require('@dashevo/dapi-grpc');

const ProcessMediator = require('../../../transactionsFilter/ProcessMediator');

const wait = require('../../../utils/wait');

/**
 * Prepare the response and send transactions response
 *
 * @param {AcknowledgingWritable} call
 * @param {Transaction[]} transactions
 * @returns {Promise<void>}
 */
async function sendTransactionsResponse(call, transactions) {
  const rawTransactions = new RawTransactions();
  rawTransactions.setTransactionsList(
    transactions.map(tx => tx.toBuffer()),
  );

  const response = new TransactionsWithProofsResponse();
  response.setRawTransactions(rawTransactions);

  await call.write(response);
}

/**
 * Prepare the response and send merkle block response
 *
 * @param {AcknowledgingWritable} call
 * @param {MerkleBlock} merkleBlock
 * @returns {Promise<void>}
 */
async function sendMerkleBlockResponse(call, merkleBlock) {
  const response = new TransactionsWithProofsResponse();
  response.setRawMerkleBlock(merkleBlock.toBuffer());

  await call.write(response);
}

/**
 * Prepare the response and send transactions response
 *
 * @param {AcknowledgingWritable} call
 * @param {InstantLock} instantLock
 * @returns {Promise<void>}
 */
async function sendInstantLockResponse(call, instantLock) {
  const instantSendLockMessages = new InstantSendLockMessages();
  instantSendLockMessages.setMessagesList([instantLock.toBuffer()]);

  const response = new TransactionsWithProofsResponse();
  response.setInstantSendLockMessages(instantSendLockMessages);

  await call.write(response);
}

/**
 *
 * @param {getHistoricalTransactionsIterator} getHistoricalTransactionsIterator
 * @param {subscribeToNewTransactions} subscribeToNewTransactions
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 * @param {testFunction} testTransactionAgainstFilter
 * @param {CoreRpcClient} coreAPI
 * @param {getMemPoolTransactions} getMemPoolTransactions
 * @return {subscribeToTransactionsWithProofsHandler}
 */
function subscribeToTransactionsWithProofsHandlerFactory(
  getHistoricalTransactionsIterator,
  subscribeToNewTransactions,
  bloomFilterEmitterCollection,
  testTransactionAgainstFilter,
  coreAPI,
  getMemPoolTransactions,
) {
  /**
   * @typedef subscribeToTransactionsWithProofsHandler
   * @param {grpc.ServerWriteableStream<TransactionsWithProofsRequest>} call
   */
  async function subscribeToTransactionsWithProofsHandler(call) {
    const { request } = call;

    const bloomFilterMessage = request.getBloomFilter();

    const bloomFilter = {
      vData: bloomFilterMessage.getVData_asU8(),
      nHashFuncs: bloomFilterMessage.getNHashFuncs(),
      nTweak: bloomFilterMessage.getNTweak(),
      nFlags: bloomFilterMessage.getNFlags(),
    };

    let fromBlockHash = Buffer.from(request.getFromBlockHash()).toString('hex');
    const fromBlockHeight = request.getFromBlockHeight();
    const count = request.getCount();

    // Create a new bloom filter emitter when client connects
    let filter;
    try {
      filter = new BloomFilter(bloomFilter);
    } catch (e) {
      throw new InvalidArgumentGrpcError(`Invalid bloom filter: ${e.message}`);
    }

    const isNewTransactionsRequested = count === 0;

    const acknowledgingCall = new AcknowledgingWritable(call);

    const mediator = new ProcessMediator();

    mediator.on(
      ProcessMediator.EVENTS.TRANSACTION,
      async (tx) => {
        await sendTransactionsResponse(acknowledgingCall, [tx]);
      },
    );

    mediator.on(
      ProcessMediator.EVENTS.MERKLE_BLOCK,
      async (merkleBlock) => {
        await sendMerkleBlockResponse(acknowledgingCall, merkleBlock);
      },
    );

    mediator.on(
      ProcessMediator.EVENTS.INSTANT_LOCK,
      async (instantLock) => {
        await sendInstantLockResponse(acknowledgingCall, instantLock);
      },
    );

    if (isNewTransactionsRequested) {
      subscribeToNewTransactions(
        mediator,
        filter,
        testTransactionAgainstFilter,
        bloomFilterEmitterCollection,
      );
    }

    // If block height is specified instead of block hash, we obtain block hash by block height
    if (fromBlockHash === '') {
      if (fromBlockHeight === 0) {
        throw new InvalidArgumentGrpcError('minimum value for `fromBlockHeight` is 1');
      }

      // we don't need to check bestBlockHeight because getBlockHash throws
      // an error in case of wrong height
      try {
        fromBlockHash = await coreAPI.getBlockHash(fromBlockHeight);
      } catch (e) {
        if (e.code === -8) {
          // Block height out of range
          throw new InvalidArgumentGrpcError('fromBlockHeight is bigger than block count');
        }

        throw e;
      }
    }

    // Send historical transactions
    let fromBlock;

    try {
      fromBlock = await coreAPI.getBlock(fromBlockHash);
    } catch (e) {
      // Block not found
      if (e.code === -5) {
        throw new InvalidArgumentGrpcError('fromBlockHash is not found');
      }

      throw e;
    }

    if (fromBlock.confirmations === -1) {
      throw new InvalidArgumentGrpcError(`block ${fromBlockHash} is not part of the best block chain`);
    }

    const bestBlockHeight = await coreAPI.getBestBlockHeight();

    let historicalCount = count;

    // if block 'count' is 0 (new transactions are requested)
    // or 'count' is bigger than chain tip we need to read all blocks
    // from specified block hash including the most recent one
    //
    // Theoretically, if count is bigger than chain tips,
    // we should throw an error 'count is too big',
    // however at the time of writing this logic, height chain sync isn't yet implemented,
    // so the client library doesn't know the exact height and
    // may pass count number larger than expected.
    // This condition should be converted to throwing an error once
    // the header stream is implemented
    if (count === 0 || fromBlock.height + count > bestBlockHeight + 1) {
      historicalCount = bestBlockHeight - fromBlock.height + 1;
    }

    const historicalDataIterator = getHistoricalTransactionsIterator(
      filter,
      fromBlockHash,
      historicalCount,
    );

    for await (const { merkleBlock, transactions, index } of historicalDataIterator) {
      if (index > 0) {
        // Wait a second between the calls to Core just to reduce the load
        await wait(50);
      }

      await sendTransactionsResponse(acknowledgingCall, transactions);
      await sendMerkleBlockResponse(acknowledgingCall, merkleBlock);

      if (isNewTransactionsRequested) {
        // removing sent transactions and blocks from cache
        mediator.emit(ProcessMediator.EVENTS.HISTORICAL_BLOCK_SENT, merkleBlock.header.hash);
      }
    }

    // notify new txs listener that we've sent historical data
    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    if (isNewTransactionsRequested) {
      // Read and test transactions from mempool
      const memPoolTransactions = await getMemPoolTransactions();
      memPoolTransactions.forEach(
        bloomFilterEmitterCollection.test.bind(bloomFilterEmitterCollection),
      );

      mediator.emit(ProcessMediator.EVENTS.MEMPOOL_DATA_SENT);
    } else {
      // End stream if user asked only for historical data
      call.end();
    }

    call.on('cancelled', () => {
      call.end();

      // remove bloom filter emitter
      mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);
    });
  }

  return subscribeToTransactionsWithProofsHandler;
}

module.exports = subscribeToTransactionsWithProofsHandlerFactory;
