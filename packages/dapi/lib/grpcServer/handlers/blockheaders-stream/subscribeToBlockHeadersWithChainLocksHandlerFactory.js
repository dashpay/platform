const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      NotFoundGrpcError,
    },
    stream: {
      AcknowledgingWritable,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BlockHeadersWithChainLocksResponse,
    BlockHeaders,
  },
} = require('@dashevo/dapi-grpc');
const ProcessMediator = require('../../../transactionsFilter/ProcessMediator');
const wait = require('../../../utils/wait');

/**
 * Prepare and send block headers response
 *
 * @param {AcknowledgingWritable} call
 * @param {BlockHeader[]} blockHeaders
 * @returns {Promise<void>}
 */
async function sendBlockHeadersResponse(call, blockHeaders) {
  const blockHeadersProto = new BlockHeaders();
  blockHeadersProto.setHeadersList(
    blockHeaders.map((blockHeader) => blockHeader.toBuffer()),
  );

  const response = new BlockHeadersWithChainLocksResponse();
  response.setBlockHeaders(blockHeadersProto);

  await call.write(response);
}

/**
 * @param {getHistoricalBlockHeadersIterator} getHistoricalBlockHeadersIterator
 * @param {CoreRpcClient} coreAPI
 * @return {subscribeToBlockHeadersWithChainLocksHandler}
 */
function subscribeToBlockHeadersWithChainLocksHandlerFactory(
  getHistoricalBlockHeadersIterator,
  coreAPI,
) {
  /**
   * @typedef subscribeToBlockHeadersWithChainLocksHandler
   * @param {grpc.ServerWriteableStream<BlockHeadersWithChainLocksRequest>} call
   */
  async function subscribeToBlockHeadersWithChainLocksHandler(call) {
    const { request } = call;

    let fromBlockHash = Buffer.from(request.getFromBlockHash()).toString('hex');
    const fromBlockHeight = request.getFromBlockHeight();
    const count = request.getCount();

    // const isNewTransactionsRequested = count === 0;

    const acknowledgingCall = new AcknowledgingWritable(call);

    const mediator = new ProcessMediator();

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
          throw new NotFoundGrpcError('fromBlockHeight is bigger than block count');
        }

        throw e;
      }
    }

    let fromBlock;

    try {
      fromBlock = await coreAPI.getBlock(fromBlockHash);
    } catch (e) {
      // Block not found
      if (e.code === -5) {
        throw new NotFoundGrpcError('fromBlockHash is not found');
      }

      throw e;
    }

    if (fromBlock.confirmations === -1) {
      throw new NotFoundGrpcError(`block ${fromBlockHash} is not part of the best block chain`);
    }

    const bestBlockHeight = await coreAPI.getBestBlockHeight();

    let historicalCount = count;

    // if block 'count' is 0 (new block headers are requested)
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

    const historicalDataIterator = getHistoricalBlockHeadersIterator(
      fromBlockHash,
      historicalCount,
    );

    for await (const blockHeaders of historicalDataIterator) {
      // Wait between the calls to Core just to reduce the load
      await wait(50);

      await sendBlockHeadersResponse(acknowledgingCall, blockHeaders);
      //
      // if (isNewTransactionsRequested) {
      //   removing sent transactions and blocks from cache
      // mediator.emit(ProcessMediator.EVENTS.HISTORICAL_BLOCK_SENT, merkleBlock.header.hash);
      // }
    }

    // notify new txs listener that we've sent historical data
    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    call.end();

    call.on('cancelled', () => {
      call.end();

      // remove bloom filter emitter
      mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);
    });
  }

  return subscribeToBlockHeadersWithChainLocksHandler;
}

module.exports = subscribeToBlockHeadersWithChainLocksHandlerFactory;
