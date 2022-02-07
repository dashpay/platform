const {ChainLock} = require('@dashevo/dashcore-lib');
const chainlocks = require('./chainlocks')

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
const ProcessMediator = require('./ProcessMediator');
const wait = require('../../../utils/wait');
const log = require('../../../log');

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
 * Prepare and send chain lock response
 *
 * @param {AcknowledgingWritable} call
 * @param {ChainLock} chainLock
 * @returns {Promise<void>}
 */
async function sendChainLockResponse(call, chainLock) {
  const response = new BlockHeadersWithChainLocksResponse();
  response.setChainLock(chainLock.toBuffer());

  await call.write(response);
}

/**
 * @param {getHistoricalBlockHeadersIterator} getHistoricalBlockHeadersIterator
 * @param {CoreRpcClient} coreAPI
 * @param {ZmqClient} zmqClient
 * @param {subscribeToNewBlockHeaders} subscribeToNewBlockHeaders
 * @param appMediator {AppMediator}
 * @return {subscribeToBlockHeadersWithChainLocksHandler}
 */
function subscribeToBlockHeadersWithChainLocksHandlerFactory(
  getHistoricalBlockHeadersIterator,
  coreAPI,
  zmqClient,
  subscribeToNewBlockHeaders,
  appMediator,
) {
  /**
   * @typedef subscribeToBlockHeadersWithChainLocksHandler
   * @param {grpc.ServerWriteableStream<BlockHeadersWithChainLocksRequest>} call
   */
  async function subscribeToBlockHeadersWithChainLocksHandler(call) {
    const { request } = call;

    const fromBlockHash = Buffer.from(request.getFromBlockHash_asU8()).toString('hex');
    const fromBlockHeight = request.getFromBlockHeight();

    if (!fromBlockHash && fromBlockHeight === 0) {
      throw new InvalidArgumentGrpcError('Minimum value for `fromBlockHeight` is 1');
    }

    const from = fromBlockHash || fromBlockHeight;
    const count = request.getCount();

    const newHeadersRequested = count === 0;

    const acknowledgingCall = new AcknowledgingWritable(call);

    const mediator = new ProcessMediator();

    mediator.on(
      ProcessMediator.EVENTS.BLOCK_HEADERS,
      async (blockHeaders) => {
        await sendBlockHeadersResponse(acknowledgingCall, blockHeaders);
      },
    );

    mediator.on(
      ProcessMediator.EVENTS.CHAIN_LOCK,
      async (chainLock) => {
        await sendChainLockResponse(acknowledgingCall, chainLock);
      },
    );

    if (newHeadersRequested) {
      subscribeToNewBlockHeaders(mediator, zmqClient, coreAPI);
    }

    let fromBlock;

    try {
      fromBlock = await coreAPI.getBlockStats(from, ['height']);
    } catch (e) {
      if (e.code === -5 || e.code === -8) {
        // -5 -> invalid block height or block is not on best chain
        // -8 -> block hash not found
        throw new NotFoundGrpcError(`Block ${from} not found`);
      }
      throw e;
    }

    const bestBlockHeight = await coreAPI.getBestBlockHeight();

    const historicalCount = count === 0 ? bestBlockHeight - fromBlock.height + 1 : count;

    if (fromBlock.height + historicalCount > bestBlockHeight + 1) {
      throw new InvalidArgumentGrpcError('`count` value exceeds the chain tip');
    }

    const bestChainLock = chainlocks.getBestChainLock()

    if (bestChainLock) {
      await sendChainLockResponse(acknowledgingCall, new ChainLock(bestChainLock));
    } else {
      log.info('No chain lock available in dashcore node');
    }

    const historicalDataIterator = getHistoricalBlockHeadersIterator(
      fromBlock.height,
      historicalCount,
    );

    for await (const blockHeaders of historicalDataIterator) {
      // Wait between the calls to Core just to reduce the load
      await wait(50);

      await sendBlockHeadersResponse(acknowledgingCall, blockHeaders);

      if (newHeadersRequested) {
        // removing sent headers from cache
        mediator.emit(
          ProcessMediator.EVENTS.HISTORICAL_BLOCK_HEADERS_SENT,
          blockHeaders.map((header) => header.hash),
        );
      }
    }

    // notify new block headers listener that we've sent historical data
    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    if (!newHeadersRequested) {
      call.end();
    }

    call.on('cancelled', () => {
      call.end();
      mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);
    });
  }

  return subscribeToBlockHeadersWithChainLocksHandler;
}

module.exports = subscribeToBlockHeadersWithChainLocksHandlerFactory;
