const {
  client: {
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
  },
  server: {
    jsonToProtobufHandlerWrapper,
    error: {
      wrapInErrorHandlerFactory,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BroadcastTransactionRequest,
    GetTransactionRequest,
    GetCoreChainStatusRequest,
    GetMasternodeStatusRequest,
    GetBlockRequest,
    pbjs: {
      BroadcastTransactionRequest: PBJSBroadcastTransactionRequest,
      BroadcastTransactionResponse: PBJSBroadcastTransactionResponse,
      GetTransactionRequest: PBJSGetTransactionRequest,
      GetTransactionResponse: PBJSGetTransactionResponse,
      GetCoreChainStatusRequest: PBJSGetCoreChainStatusRequest,
      GetCoreChainStatusResponse: PBJSGetCoreChainStatusResponse,
      GetMasternodeStatusRequest: PBJSGetMasternodeStatusRequest,
      GetMasternodeStatusResponse: PBJSGetMasternodeStatusResponse,
      GetBlockRequest: PBJSGetBlockRequest,
      GetBlockResponse: PBJSGetBlockResponse,
    },
  },
} = require('@dashevo/dapi-grpc');

const logger = require('../../../logger');

const getBlockHandlerFactory = require(
  './getBlockHandlerFactory',
);
const getCoreChainStatusHandlerFactory = require(
  './getCoreChainStatusHandlerFactory',
);
const getMasternodeStatusHandlerFactory = require(
  './getMasternodeStatusHandlerFactory',
);
const getTransactionHandlerFactory = require(
  './getTransactionHandlerFactory',
);
const broadcastTransactionHandlerFactory = require(
  './broadcastTransactionHandlerFactory',
);

/**
 * @param {CoreRpcClient} coreRPCClient
 * @param {boolean} isProductionEnvironment
 * @returns {Object<string, function>}
 */
function coreHandlersFactory(coreRPCClient, isProductionEnvironment) {
  const wrapInErrorHandler = wrapInErrorHandlerFactory(logger, isProductionEnvironment);

  // getBlock
  const getBlockHandler = getBlockHandlerFactory(coreRPCClient);
  const wrappedGetBlock = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetBlockRequest,
      PBJSGetBlockRequest,
    ),
    protobufToJsonFactory(
      PBJSGetBlockResponse,
    ),
    wrapInErrorHandler(getBlockHandler),
  );

  // getCoreChainStatus
  const getCoreChainStatusHandler = getCoreChainStatusHandlerFactory(coreRPCClient);
  const wrappedGetCoreChainStatus = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetCoreChainStatusRequest,
      PBJSGetCoreChainStatusRequest,
    ),
    protobufToJsonFactory(
      PBJSGetCoreChainStatusResponse,
    ),
    wrapInErrorHandler(getCoreChainStatusHandler),
  );

  // getMasternodeStatus
  const getMasternodeStatusHandler = getMasternodeStatusHandlerFactory(coreRPCClient);
  const wrappedGetMasternodeStatus = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetMasternodeStatusRequest,
      PBJSGetMasternodeStatusRequest,
    ),
    protobufToJsonFactory(
      PBJSGetMasternodeStatusResponse,
    ),
    wrapInErrorHandler(getMasternodeStatusHandler),
  );

  // getTransaction
  const getTransactionHandler = getTransactionHandlerFactory(coreRPCClient);
  const wrappedGetTransaction = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetTransactionRequest,
      PBJSGetTransactionRequest,
    ),
    protobufToJsonFactory(
      PBJSGetTransactionResponse,
    ),
    wrapInErrorHandler(getTransactionHandler),
  );

  // broadcastTransaction
  const broadcastTransactionHandler = broadcastTransactionHandlerFactory(coreRPCClient);
  const wrappedBroadcastTransaction = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      BroadcastTransactionRequest,
      PBJSBroadcastTransactionRequest,
    ),
    protobufToJsonFactory(
      PBJSBroadcastTransactionResponse,
    ),
    wrapInErrorHandler(broadcastTransactionHandler),
  );

  return {
    getBlock: wrappedGetBlock,
    getCoreChainStatus: wrappedGetCoreChainStatus,
    getMasternodeStatus: wrappedGetMasternodeStatus,
    getTransaction: wrappedGetTransaction,
    broadcastTransaction: wrappedBroadcastTransaction,
  };
}

module.exports = coreHandlersFactory;
