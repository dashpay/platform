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
 *
 * @param {getHistoricalTransactionsIterator} getHistoricalTransactionsIterator
 * @param {subscribeToNewTransactions} subscribeToNewTransactions
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 * @param {testFunction} testTransactionAgainstFilter
 * @param {CoreRpcClient} coreAPI
 * @return {subscribeToTransactionsWithProofsHandler}
 */
function subscribeToTransactionsWithProofsHandlerFactory(
  getHistoricalTransactionsIterator,
  subscribeToNewTransactions,
  bloomFilterEmitterCollection,
  testTransactionAgainstFilter,
  coreAPI,
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

    const fromBlockHash = Buffer.from(request.getFromBlockHash()).toString('hex');
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

    let blockHash = fromBlockHash;

    if (blockHash === '') {
      if (fromBlockHeight === 0) {
        throw new InvalidArgumentGrpcError('minimum value for `fromBlockHeight` is 1');
      }

      const bestHeight = await coreAPI.getBestBlockHeight();

      if (fromBlockHeight > bestHeight) {
        throw new InvalidArgumentGrpcError('fromBlockHeight is bigger than block count');
      }

      blockHash = await coreAPI.getBlockHash(fromBlockHeight);
    }

    if (count > 0) {
      const block = await coreAPI.getBlock(blockHash);
      const bestBlockHeight = await coreAPI.getBestBlockHeight();

      if (block.height + count > bestBlockHeight + 1) {
        throw new InvalidArgumentGrpcError(
          'count is too big, could not fetch more than blockchain length',
        );
      }
    }

    const acknowledgingCall = new AcknowledgingWritable(call);
    const mediator = new ProcessMediator();

    if (isNewTransactionsRequested) {
      subscribeToNewTransactions(
        mediator,
        filter,
        testTransactionAgainstFilter,
        bloomFilterEmitterCollection,
      );
    }

    let historicalCount = count;
    if (subscribeToNewTransactions) {
      const block = await coreAPI.getBlock(blockHash);
      const bestBlockHeight = await coreAPI.getBestBlockHeight();

      historicalCount = bestBlockHeight - block.height + 1;
    }

    const historicalDataIterator = getHistoricalTransactionsIterator(
      filter,
      blockHash,
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

    if (isNewTransactionsRequested) {
      // new txs listener will send us unsent cached data back
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

      // notify new txs listener that we've sent historical data
      mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);
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
