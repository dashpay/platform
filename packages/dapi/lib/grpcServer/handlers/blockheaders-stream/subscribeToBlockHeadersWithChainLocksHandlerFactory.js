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
    ChainLockSignatureMessages,
  },
} = require('@dashevo/dapi-grpc');
const ProcessMediator = require('./ProcessMediator');
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
 * Prepare and send clSig response
 *
 * @param {AcknowledgingWritable} call
 * @param {Buffer} clSig
 * @returns {Promise<void>}
 */
async function sendClSigResponse(call, clSig) {
  const clSigMessages = new ChainLockSignatureMessages();
  clSigMessages.setMessagesList([clSig]);

  const response = new BlockHeadersWithChainLocksResponse();
  response.setChainLockSignatureMessages(clSigMessages);

  await call.write(response);
}

/**
 * @param {getHistoricalBlockHeadersIterator} getHistoricalBlockHeadersIterator
 * @param {CoreRpcClient} coreAPI
 * @param {ZmqClient} zmqClient
 * @param {subscribeToNewBlockHeaders} subscribeToNewBlockHeaders
 * @return {subscribeToBlockHeadersWithChainLocksHandler}
 */
function subscribeToBlockHeadersWithChainLocksHandlerFactory(
  getHistoricalBlockHeadersIterator,
  coreAPI,
  zmqClient,
  subscribeToNewBlockHeaders,
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
      ProcessMediator.EVENTS.CHAIN_LOCK_SIGNATURE,
      async (clSig) => {
        await sendClSigResponse(acknowledgingCall, clSig);
      },
    );

    if (newHeadersRequested) {
      subscribeToNewBlockHeaders(mediator, zmqClient, coreAPI);
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

    const historicalCount = count === 0 ? bestBlockHeight - fromBlock.height + 1 : count;

    if (fromBlock.height + historicalCount > bestBlockHeight + 1) {
      throw new InvalidArgumentGrpcError('`count` value exceeds the chain tip');
    }

    const bestChainLock = await coreAPI.getBestChainLock();
    await sendClSigResponse(acknowledgingCall, Buffer.from(bestChainLock.signature, 'hex'));

    const historicalDataIterator = getHistoricalBlockHeadersIterator(
      fromBlockHash,
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

    // notify new txs listener that we've sent historical data
    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    if (!newHeadersRequested) {
      call.end();
    }

    call.on('cancelled', () => {
      call.end();

      // remove bloom filter emitter
      mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);
    });
  }

  return subscribeToBlockHeadersWithChainLocksHandler;
}

module.exports = subscribeToBlockHeadersWithChainLocksHandlerFactory;
