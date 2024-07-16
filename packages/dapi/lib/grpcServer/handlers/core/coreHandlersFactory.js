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
    GetBlockchainStatusRequest,
    // GetMasternodeStatusRequest,
    // GetBlockRequest,
    GetBestBlockHeightRequest,
    pbjs: {
      BroadcastTransactionRequest: PBJSBroadcastTransactionRequest,
      BroadcastTransactionResponse: PBJSBroadcastTransactionResponse,
      GetTransactionRequest: PBJSGetTransactionRequest,
      GetTransactionResponse: PBJSGetTransactionResponse,
      GetBlockchainStatusRequest: PBJSGetBlockchainStatusRequest,
      GetBlockchainStatusResponse: PBJSGetBlockchainStatusResponse,
      // GetMasternodeStatusRequest: PBJSGetMasternodeStatusRequest,
      // GetMasternodeStatusResponse: PBJSGetMasternodeStatusResponse,
      // GetBlockRequest: PBJSGetBlockRequest,
      // GetBlockResponse: PBJSGetBlockResponse,
      GetBestBlockHeightRequest: PBJSGetBestBlockHeightRequest,
      GetBestBlockHeightResponse: PBJSGetBestBlockHeightResponse,
    },
  },
} = require('@dashevo/dapi-grpc');

const logger = require('../../../logger');

// const getBlockHandlerFactory = require(
//   './getBlockHandlerFactory',
// );
const getBestBlockHeightHandlerFactory = require(
  './getBestBlockHeightHandlerFactory',
);
const getBlockchainStatusHandlerFactory = require(
  './getBlockchainStatusHandlerFactory',
);
// const getMasternodeStatusHandlerFactory = require(
//   './getMasternodeStatusHandlerFactory',
// );
const getTransactionHandlerFactory = require(
  './getTransactionHandlerFactory',
);
const broadcastTransactionHandlerFactory = require(
  './broadcastTransactionHandlerFactory',
);

/**
 * @param {CoreRpcClient} coreRPCClient
 * @param {boolean} isProductionEnvironment
 * @param {ZmqClient} coreZmqClient
 * @returns {Object<string, function>}
 */
function coreHandlersFactory(coreRPCClient, isProductionEnvironment, coreZmqClient) {
  const wrapInErrorHandler = wrapInErrorHandlerFactory(logger, isProductionEnvironment);

  // getBlock
  // const getBlockHandler = getBlockHandlerFactory(coreRPCClient);
  // const wrappedGetBlock = jsonToProtobufHandlerWrapper(
  //   jsonToProtobufFactory(
  //     GetBlockRequest,
  //     PBJSGetBlockRequest,
  //   ),
  //   protobufToJsonFactory(
  //     PBJSGetBlockResponse,
  //   ),
  //   wrapInErrorHandler(getBlockHandler),
  // );

  // getBestBlockHeight
  const getBestBlockHeightHandler = getBestBlockHeightHandlerFactory(coreRPCClient, coreZmqClient);
  const wrappedGetBestBlockHeightHandler = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetBestBlockHeightRequest,
      PBJSGetBestBlockHeightRequest,
    ),
    protobufToJsonFactory(
      PBJSGetBestBlockHeightResponse,
    ),
    wrapInErrorHandler(getBestBlockHeightHandler),
  );

  // getBlockchainStatus
  const getBlockchainStatusHandler = getBlockchainStatusHandlerFactory(
    coreRPCClient,
    coreZmqClient,
  );
  const wrappedGetBlockchainStatus = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetBlockchainStatusRequest,
      PBJSGetBlockchainStatusRequest,
    ),
    protobufToJsonFactory(
      PBJSGetBlockchainStatusResponse,
    ),
    wrapInErrorHandler(getBlockchainStatusHandler),
  );

  // getMasternodeStatus
  // const getMasternodeStatusHandler = getMasternodeStatusHandlerFactory(coreRPCClient);
  // const wrappedGetMasternodeStatus = jsonToProtobufHandlerWrapper(
  //   jsonToProtobufFactory(
  //     GetMasternodeStatusRequest,
  //     PBJSGetMasternodeStatusRequest,
  //   ),
  //   protobufToJsonFactory(
  //     PBJSGetMasternodeStatusResponse,
  //   ),
  //   wrapInErrorHandler(getMasternodeStatusHandler),
  // );

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
    // TODO: Enable when an attack resistance is proved
    // getBlock: wrappedGetBlock,
    getBestBlockHeight: wrappedGetBestBlockHeightHandler,
    getBlockchainStatus: wrappedGetBlockchainStatus,
    // getMasternodeStatus: wrappedGetMasternodeStatus,
    getTransaction: wrappedGetTransaction,
    broadcastTransaction: wrappedBroadcastTransaction,
  };
}

module.exports = coreHandlersFactory;
