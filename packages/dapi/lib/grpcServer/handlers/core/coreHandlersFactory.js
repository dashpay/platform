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
    GetStatusRequest,
    GetBlockRequest,
    pbjs: {
      BroadcastTransactionRequest: PBJSBroadcastTransactionRequest,
      BroadcastTransactionResponse: PBJSBroadcastTransactionResponse,
      GetTransactionRequest: PBJSGetTransactionRequest,
      GetTransactionResponse: PBJSGetTransactionResponse,
      GetStatusRequest: PBJSGetStatusRequest,
      GetStatusResponse: PBJSGetStatusResponse,
      GetBlockRequest: PBJSGetBlockRequest,
      GetBlockResponse: PBJSGetBlockResponse,
    },
  },
} = require('@dashevo/dapi-grpc');

const log = require('../../../log');

const getBlockHandlerFactory = require(
  './getBlockHandlerFactory',
);
const getStatusHandlerFactory = require(
  './getStatusHandlerFactory',
);
const getTransactionHandlerFactory = require(
  './getTransactionHandlerFactory',
);
const broadcastTransactionHandlerFactory = require(
  './broadcastTransactionHandlerFactory',
);

/**
 * @param {InsightAPI} insightAPI
 * @param {boolean} isProductionEnvironment
 * @returns {Object<string, function>}
 */
function coreHandlersFactory(insightAPI, isProductionEnvironment) {
  const wrapInErrorHandler = wrapInErrorHandlerFactory(log, isProductionEnvironment);

  // getBlock
  const getBlockHandler = getBlockHandlerFactory(insightAPI);
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

  // getStatus
  const getStatusHandler = getStatusHandlerFactory(insightAPI);
  const wrappedGetStatus = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetStatusRequest,
      PBJSGetStatusRequest,
    ),
    protobufToJsonFactory(
      PBJSGetStatusResponse,
    ),
    wrapInErrorHandler(getStatusHandler),
  );

  // getTransaction
  const getTransactionHandler = getTransactionHandlerFactory(insightAPI);
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
  const broadcastTransactionHandler = broadcastTransactionHandlerFactory(insightAPI);
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
    getStatus: wrappedGetStatus,
    getTransaction: wrappedGetTransaction,
    broadcastTransaction: wrappedBroadcastTransaction,
  };
}

module.exports = coreHandlersFactory;
